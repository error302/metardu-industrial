// Shoal-Biased Sounding Generalization Engine.
// Filters high-density soundings to generate clean nautical chart depth labels
// based on target chart scale, preserving shoalest critical depths.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoundingPoint {
    pub id: u64,
    pub x: f64,
    pub y: f64,
    pub depth_m: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartSoundingSelectionParams {
    pub target_scale_denominator: u32, // e.g. 5000 for 1:5,000 scale
    pub min_spacing_mm_on_chart: f64,   // e.g. 10.0 mm minimum text label separation
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectedChartSoundings {
    pub selected_soundings: Vec<SoundingPoint>,
    pub total_input_count: usize,
    pub selected_count: usize,
    pub suppression_ratio_pct: f64,
}

/// Perform Shoal-Biased Sounding Suppression.
/// Sorts soundings by depth (shallowest first) and selects points ensuring min spatial buffer.
pub fn select_shoal_biased_soundings(
    soundings: &[SoundingPoint],
    params: &ChartSoundingSelectionParams,
) -> SelectedChartSoundings {
    if soundings.is_empty() {
        return SelectedChartSoundings {
            selected_soundings: vec![],
            total_input_count: 0,
            selected_count: 0,
            suppression_ratio_pct: 0.0,
        };
    }

    // Minimum ground distance buffer in meters:
    // Ground buffer = (min_spacing_mm / 1000.0) * target_scale
    let ground_buffer_m = (params.min_spacing_mm_on_chart / 1000.0) * (params.target_scale_denominator as f64);
    let buffer_sq = ground_buffer_m * ground_buffer_m;

    // 1. Sort soundings shoal-first (smallest depth_m first)
    let mut sorted = soundings.to_vec();
    sorted.sort_by(|a, b| a.depth_m.partial_cmp(&b.depth_m).unwrap_or(std::cmp::Ordering::Equal));

    // 2. Greedy spatial buffer selection
    let mut selected: Vec<SoundingPoint> = Vec::new();

    for candidate in sorted {
        let mut conflicts = false;
        for existing in &selected {
            let dx = candidate.x - existing.x;
            let dy = candidate.y - existing.y;
            if (dx * dx + dy * dy) < buffer_sq {
                conflicts = true;
                break;
            }
        }

        if !conflicts {
            selected.push(candidate);
        }
    }

    let selected_count = selected.len();
    let total_count = soundings.len();
    let suppression = if total_count > 0 {
        ((total_count - selected_count) as f64 / total_count as f64) * 100.0
    } else {
        0.0
    };

    SelectedChartSoundings {
        selected_soundings: selected,
        total_input_count: total_count,
        selected_count,
        suppression_ratio_pct: suppression,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shoal_biased_selection() {
        let pts = vec![
            SoundingPoint { id: 1, x: 100.0, y: 100.0, depth_m: 12.5 }, // Shallow
            SoundingPoint { id: 2, x: 101.0, y: 100.0, depth_m: 25.0 }, // Deep (within 5m buffer -> should be suppressed)
            SoundingPoint { id: 3, x: 200.0, y: 200.0, depth_m: 15.0 }, // Far away -> selected
        ];

        let params = ChartSoundingSelectionParams {
            target_scale_denominator: 1000,
            min_spacing_mm_on_chart: 10.0, // 10m ground buffer
        };

        let result = select_shoal_biased_soundings(&pts, &params);
        assert_eq!(result.selected_count, 2);
        assert_eq!(result.selected_soundings[0].id, 1); // Shallowest point #1 selected
    }
}
