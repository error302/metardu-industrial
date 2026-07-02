// TPU (Total Propagated Uncertainty) — pure Rust.
//
// Reference: Hare et al. (1995) "Noise estimation techniques for
// swath bathymetry"
// IHO S-44 (6th edition, 2022) §3.5 uncertainty budget requirements.
//
// TPU combines all sources of vertical uncertainty in a sounding:
//   - Sensor uncertainties (sonar beam angle, range, attitude sensor noise)
//   - Attitude uncertainties (roll, pitch, yaw, heave, latency)
//   - SVP uncertainties (sound speed cast accuracy, temporal/spatial
//     representativeness)
//   - Tide and water level uncertainties (gauge accuracy, zoning errors)
//   - Coordinate transformation uncertainties (datum shift residuals)
//
// The combined vertical TPU is compared against S-44 order thresholds.
// This Phase 2 implementation uses the standard error propagation
// formula: TPU = sqrt(sum of individual variance contributions).
//
// Each uncertainty source is specified as 1-sigma. The combined TPU
// is also 1-sigma. For S-44 compliance, we compare against the order
// thresholds at 95% confidence level (2 * sigma).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TpuComponents {
    // Sensor uncertainties (1-sigma)
    pub beam_angle_sigma: f64,       // radians
    pub range_sigma: f64,            // meters
    pub attitude_roll_sigma: f64,    // radians
    pub attitude_pitch_sigma: f64,   // radians
    pub attitude_yaw_sigma: f64,     // radians
    pub attitude_heave_sigma: f64,   // meters
    pub attitude_latency_sigma: f64, // seconds

    // SVP uncertainty
    pub svp_sigma: f64, // m/s

    // Tide/water level uncertainty
    pub tide_sigma: f64, // meters

    // Coordinate transformation residual
    pub datum_sigma: f64, // meters
}

impl Default for TpuComponents {
    fn default() -> Self {
        // Typical values for a modern multibeam system (EM 710 class)
        Self {
            beam_angle_sigma: 0.0005,      // 0.03 degrees
            range_sigma: 0.02,             // 2cm
            attitude_roll_sigma: 0.00035,  // 0.02 degrees
            attitude_pitch_sigma: 0.00035, // 0.02 degrees
            attitude_yaw_sigma: 0.00070,   // 0.04 degrees
            attitude_heave_sigma: 0.05,    // 5cm
            attitude_latency_sigma: 0.001, // 1ms
            svp_sigma: 0.5,                // 0.5 m/s
            tide_sigma: 0.05,              // 5cm
            datum_sigma: 0.02,             // 2cm
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoundingTpuInput {
    /// Beam angle from nadir (radians, 0 = straight down)
    pub beam_angle: f64,
    /// Two-way travel time (seconds)
    pub travel_time: f64,
    /// Mean sound speed along the ray path (m/s)
    pub sound_speed: f64,
    /// Depth below transducer (meters)
    pub depth: f64,
    /// Component uncertainties
    pub components: TpuComponents,
}

#[derive(Debug, Clone, Serialize)]
pub struct TpuResult {
    /// Vertical TPU at 1-sigma (meters)
    pub vertical_tpu_1sigma: f64,
    /// Vertical TPU at 95% confidence (2-sigma, meters)
    pub vertical_tpu_95: f64,
    /// Horizontal TPU at 1-sigma (meters)
    pub horizontal_tpu_1sigma: f64,
    /// Horizontal TPU at 95% confidence (meters)
    pub horizontal_tpu_95: f64,
    /// Individual variance contributions (for the uncertainty budget report)
    pub vertical_contributions: TpuContributions,
    pub horizontal_contributions: TpuContributions,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct TpuContributions {
    pub sensor_variance: f64,
    pub attitude_variance: f64,
    pub svp_variance: f64,
    pub tide_variance: f64,
    pub datum_variance: f64,
}

#[derive(Debug, thiserror::Error)]
pub enum TpuError {
    #[error("invalid input: {0}")]
    Invalid(String),
}

/// Compute TPU for a single sounding using error propagation.
///
/// The vertical uncertainty from each source is computed using the
/// partial derivatives of depth w.r.t. each parameter, then combined
/// in quadrature:
///   σ²_depth = Σ (∂depth/∂param)² × σ²_param
///
/// Key relationships:
///   depth = (sound_speed × travel_time / 2) × cos(beam_angle)
///   horizontal_offset = (sound_speed × travel_time / 2) × sin(beam_angle)
pub fn compute_tpu(input: &SoundingTpuInput) -> Result<TpuResult, TpuError> {
    if input.depth <= 0.0 {
        return Err(TpuError::Invalid("depth must be positive".into()));
    }
    if input.sound_speed <= 0.0 {
        return Err(TpuError::Invalid("sound_speed must be positive".into()));
    }
    if input.travel_time <= 0.0 {
        return Err(TpuError::Invalid("travel_time must be positive".into()));
    }

    let c = input.components;

    // Slant range (one-way)
    let slant_range = input.sound_speed * input.travel_time / 2.0;

    // Partial derivatives for VERTICAL TPU:
    // depth = slant_range × cos(beam_angle)
    //       = (c × t / 2) × cos(θ)

    // ∂depth/∂θ = -slant_range × sin(beam_angle) [from beam angle]
    let d_depth_d_beam = -slant_range * input.beam_angle.sin();

    // ∂depth/∂t = (c / 2) × cos(beam_angle) [from travel time]
    let d_depth_d_time = (input.sound_speed / 2.0) * input.beam_angle.cos();

    // ∂depth/∂c = (t / 2) × cos(beam_angle) [from sound speed]
    let d_depth_d_c = (input.travel_time / 2.0) * input.beam_angle.cos();

    // Attitude effects on depth:
    // Roll directly affects beam angle → ∂depth/∂roll = d_depth_d_beam
    // Pitch has similar effect for forward/aft beams
    // Heave directly adds to depth
    let d_depth_d_roll = d_depth_d_beam * c.attitude_roll_sigma.signum();
    let d_depth_d_pitch = d_depth_d_beam * 0.5; // Pitch effect is partial
    let d_depth_d_heave: f64 = 1.0; // Heave is direct

    // Vertical variance contributions
    let sensor_var = d_depth_d_beam.powi(2) * c.beam_angle_sigma.powi(2)
        + d_depth_d_time.powi(2) * (c.range_sigma / (input.sound_speed / 2.0)).powi(2);
    let attitude_var = d_depth_d_roll.powi(2) * c.attitude_roll_sigma.powi(2)
        + d_depth_d_pitch.powi(2) * c.attitude_pitch_sigma.powi(2)
        + d_depth_d_heave.powi(2) * c.attitude_heave_sigma.powi(2);
    let svp_var = d_depth_d_c.powi(2) * c.svp_sigma.powi(2);
    let tide_var = c.tide_sigma.powi(2);
    let datum_var = c.datum_sigma.powi(2);

    let vertical_tpu_1sigma = (sensor_var + attitude_var + svp_var + tide_var + datum_var).sqrt();
    let vertical_tpu_95 = 2.0 * vertical_tpu_1sigma;

    // Partial derivatives for HORIZONTAL TPU:
    // horizontal = slant_range × sin(beam_angle)
    let d_horiz_d_beam = slant_range * input.beam_angle.cos();
    let d_horiz_d_time = (input.sound_speed / 2.0) * input.beam_angle.sin();
    let d_horiz_d_c = (input.travel_time / 2.0) * input.beam_angle.sin();

    // Yaw directly affects horizontal position
    let d_horiz_d_yaw = slant_range * input.beam_angle.sin();

    let h_sensor_var = d_horiz_d_beam.powi(2) * c.beam_angle_sigma.powi(2)
        + d_horiz_d_time.powi(2) * (c.range_sigma / (input.sound_speed / 2.0)).powi(2);
    let h_attitude_var = d_horiz_d_yaw.powi(2) * c.attitude_yaw_sigma.powi(2);
    let h_svp_var = d_horiz_d_c.powi(2) * c.svp_sigma.powi(2);
    let h_datum_var = c.datum_sigma.powi(2);

    let horizontal_tpu_1sigma = (h_sensor_var + h_attitude_var + h_svp_var + h_datum_var).sqrt();
    let horizontal_tpu_95 = 2.0 * horizontal_tpu_1sigma;

    Ok(TpuResult {
        vertical_tpu_1sigma,
        vertical_tpu_95,
        horizontal_tpu_1sigma,
        horizontal_tpu_95,
        vertical_contributions: TpuContributions {
            sensor_variance: sensor_var,
            attitude_variance: attitude_var,
            svp_variance: svp_var,
            tide_variance: tide_var,
            datum_variance: datum_var,
        },
        horizontal_contributions: TpuContributions {
            sensor_variance: h_sensor_var,
            attitude_variance: h_attitude_var,
            svp_variance: h_svp_var,
            tide_variance: 0.0, // Tide doesn't affect horizontal
            datum_variance: h_datum_var,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nadir_sounding_low_tpu() {
        // Nadir beam (beam_angle=0) at 10m depth
        let input = SoundingTpuInput {
            beam_angle: 0.0,
            travel_time: 0.0133, // ~10m at 1500 m/s
            sound_speed: 1500.0,
            depth: 10.0,
            components: TpuComponents::default(),
        };
        let result = compute_tpu(&input).unwrap();
        // Nadir TPU should be small — mostly heave + tide
        assert!(
            result.vertical_tpu_1sigma < 0.2,
            "nadir TPU {} < 0.2",
            result.vertical_tpu_1sigma
        );
    }

    #[test]
    fn test_outer_beam_higher_tpu() {
        // 60-degree beam at 50m depth — at deeper water, the SVP and
        // sensor terms dominate, so outer beams should have higher TPU
        // than nadir (which is mostly heave-limited).
        let nadir = SoundingTpuInput {
            beam_angle: 0.0,
            travel_time: 0.0667, // ~50m at 1500 m/s
            sound_speed: 1500.0,
            depth: 50.0,
            components: TpuComponents::default(),
        };
        let outer = SoundingTpuInput {
            beam_angle: std::f64::consts::FRAC_PI_3, // 60 degrees
            travel_time: 0.1334,                     // longer path for 50m depth at 60°
            sound_speed: 1500.0,
            depth: 50.0,
            components: TpuComponents::default(),
        };
        let nadir_tpu = compute_tpu(&nadir).unwrap();
        let outer_tpu = compute_tpu(&outer).unwrap();
        assert!(
            outer_tpu.vertical_tpu_1sigma > nadir_tpu.vertical_tpu_1sigma,
            "outer beam TPU {} should > nadir TPU {}",
            outer_tpu.vertical_tpu_1sigma,
            nadir_tpu.vertical_tpu_1sigma
        );
    }

    #[test]
    fn test_95_confidence_is_2x_sigma() {
        let input = SoundingTpuInput {
            beam_angle: 0.5,
            travel_time: 0.015,
            sound_speed: 1500.0,
            depth: 11.25,
            components: TpuComponents::default(),
        };
        let result = compute_tpu(&input).unwrap();
        assert!((result.vertical_tpu_95 - 2.0 * result.vertical_tpu_1sigma).abs() < 1e-10);
    }
}
