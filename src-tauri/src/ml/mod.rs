// ML classification scaffold — seafloor habitat + blast fragmentation.
//
// Per ARCHITECTURE.md §9.5 — pre-trained models for seafloor habitat
// classification (from backscatter) and blast fragmentation analysis
// (from drone imagery of muck piles).
//
// Phase 3 scaffold: defines the model interface, provides feature
// extraction pipelines, and implements a simple rule-based classifier
// as a baseline. Real ML model inference via ONNX runtime is Phase 4+.

use serde::{Deserialize, Serialize};

// ──────────────────────────────────────────────────────────────────
// Seafloor habitat classification

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HabitatClass {
    Rock,
    CoarseSediment,
    Sand,
    Mud,
    Mixed,
}

impl HabitatClass {
    #[allow(dead_code)]
    pub fn label(&self) -> &'static str {
        match self {
            HabitatClass::Rock => "Rock / Hard substrate",
            HabitatClass::CoarseSediment => "Coarse sediment (gravel)",
            HabitatClass::Sand => "Sand",
            HabitatClass::Mud => "Mud / Fine sediment",
            HabitatClass::Mixed => "Mixed substrate",
        }
    }

    #[allow(dead_code)]
    pub fn color(&self) -> [u8; 3] {
        match self {
            HabitatClass::Rock => [120, 80, 40],
            HabitatClass::CoarseSediment => [200, 180, 140],
            HabitatClass::Sand => [240, 220, 100],
            HabitatClass::Mud => [60, 80, 120],
            HabitatClass::Mixed => [160, 160, 160],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackscatterFeatures {
    pub mean_intensity: f64,
    pub std_intensity: f64,
    pub angular_slope: f64,
    pub angular_curvature: f64,
    pub texture_homogeneity: f64,
    pub depth: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct HabitatClassificationResult {
    pub class: HabitatClass,
    pub confidence: f64,
    pub class_probabilities: [f64; 5],
}

/// Classify seafloor habitat from backscatter features using a simple
/// rule-based classifier (Phase 3 baseline).
pub fn classify_habitat(features: &BackscatterFeatures) -> HabitatClassificationResult {
    let mi = features.mean_intensity;

    let (class, confidence) = if mi > -10.0 && features.texture_homogeneity < 0.3 {
        (HabitatClass::Rock, 0.85)
    } else if mi > -15.0 && features.texture_homogeneity < 0.4 {
        (HabitatClass::Rock, 0.70)
    } else if mi > -25.0 && features.angular_slope > 0.3 {
        (HabitatClass::CoarseSediment, 0.75)
    } else if mi > -30.0 {
        (HabitatClass::Sand, 0.80)
    } else if mi > -38.0 {
        (HabitatClass::Mud, 0.75)
    } else {
        (HabitatClass::Mixed, 0.50)
    };

    let base_prob = confidence;
    let residual = (1.0 - base_prob) / 4.0;
    let mut probs = [residual; 5];
    let idx = match class {
        HabitatClass::Rock => 0,
        HabitatClass::CoarseSediment => 1,
        HabitatClass::Sand => 2,
        HabitatClass::Mud => 3,
        HabitatClass::Mixed => 4,
    };
    probs[idx] = base_prob;

    HabitatClassificationResult {
        class,
        confidence,
        class_probabilities: probs,
    }
}

// ──────────────────────────────────────────────────────────────────
// Blast fragmentation analysis

#[derive(Debug, Clone, Serialize)]
pub struct FragmentationResult {
    pub p20: f64,
    pub p50: f64,
    pub p80: f64,
    pub p90: f64,
    pub uniformity: f64,
    pub mean_size: f64,
    pub quality: FragmentationQuality,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FragmentationQuality {
    Excellent,
    Acceptable,
    Coarse,
    VeryCoarse,
}

/// Analyze blast fragmentation from particle size distribution data.
pub fn analyze_fragmentation(sizes_mm: &[f64]) -> Result<FragmentationResult, String> {
    if sizes_mm.is_empty() {
        return Err("empty fragment size array".into());
    }
    if sizes_mm.len() < 10 {
        return Err(format!(
            "need at least 10 fragments, got {}",
            sizes_mm.len()
        ));
    }

    let mut sorted = sizes_mm.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let n = sorted.len();
    let percentile = |p: f64| -> f64 {
        let idx = ((p / 100.0) * (n - 1) as f64).round() as usize;
        sorted[idx.min(n - 1)]
    };

    let p20 = percentile(20.0);
    let p50 = percentile(50.0);
    let p80 = percentile(80.0);
    let p90 = percentile(90.0);
    let p10 = percentile(10.0);
    let p60 = percentile(60.0);

    let mean_size: f64 = sorted.iter().sum::<f64>() / n as f64;
    let uniformity = if p10 > 0.0 { p60 / p10 } else { 1.0 };

    let quality = if p80 < 300.0 {
        FragmentationQuality::Excellent
    } else if p80 < 500.0 {
        FragmentationQuality::Acceptable
    } else if p80 < 800.0 {
        FragmentationQuality::Coarse
    } else {
        FragmentationQuality::VeryCoarse
    };

    Ok(FragmentationResult {
        p20,
        p50,
        p80,
        p90,
        uniformity,
        mean_size,
        quality,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rock_classification() {
        let features = BackscatterFeatures {
            mean_intensity: -5.0,
            std_intensity: 3.0,
            angular_slope: 0.5,
            angular_curvature: 0.01,
            texture_homogeneity: 0.2,
            depth: 15.0,
        };
        let result = classify_habitat(&features);
        assert_eq!(result.class, HabitatClass::Rock);
        assert!(result.confidence > 0.7);
    }

    #[test]
    fn test_mud_classification() {
        let features = BackscatterFeatures {
            mean_intensity: -36.0,
            std_intensity: 1.0,
            angular_slope: 0.1,
            angular_curvature: 0.001,
            texture_homogeneity: 0.8,
            depth: 50.0,
        };
        let result = classify_habitat(&features);
        assert_eq!(result.class, HabitatClass::Mud);
    }

    #[test]
    fn test_fragmentation_well_fragmented() {
        let sizes: Vec<f64> = (0..1000).map(|i| 50.0 + (i as f64 % 150.0)).collect();
        let result = analyze_fragmentation(&sizes).unwrap();
        assert_eq!(result.quality, FragmentationQuality::Excellent);
        assert!(result.p80 < 300.0);
    }

    #[test]
    fn test_fragmentation_coarse() {
        let sizes: Vec<f64> = (0..1000).map(|i| 500.0 + (i as f64 % 700.0)).collect();
        let result = analyze_fragmentation(&sizes).unwrap();
        assert!(result.p80 > 500.0);
        assert_ne!(result.quality, FragmentationQuality::Excellent);
    }

    #[test]
    fn test_fragmentation_empty_errors() {
        let result = analyze_fragmentation(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_fragmentation_too_few_errors() {
        let sizes = vec![100.0; 5];
        let result = analyze_fragmentation(&sizes);
        assert!(result.is_err());
    }
}
