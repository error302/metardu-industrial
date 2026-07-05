// EOM Pipeline Validation — runs the full pipeline on real + synthetic
// data and prints results for cross-validation.
//
// Usage:
//   cargo run --release --bin eom_validation -- /path/to/stockpile.las
//
// This binary:
//   1. Reads a LAS file using the real LAS reader
//   2. Runs the full EOM pipeline (LAS → CSF → DEM → volumes)
//   3. Prints the volume results
//   4. Compares against the analytical cone volume (3351.03 m³)

use metardu_core::mining::eom::{run_eom_pipeline, EomInput, EomProgress};
use metardu_core::mining::las;
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        PathBuf::from("/home/z/my-project/test-data/stockpile.las")
    };

    println!("═{}", "═".repeat(60));
    println!("EOM PIPELINE VALIDATION — Rust core");
    println!("═{}", "═".repeat(60));
    println!();

    // ── Step 1: Read the LAS header ──
    println!("Step 1: Reading LAS header...");
    let header = match las::read_header(&path) {
        Ok(h) => {
            println!("  ✓ LAS {}.{}", h.version_major, h.version_minor);
            println!("    Points: {}", h.num_point_records);
            println!(
                "    Bounds: X[{:.1}, {:.1}] Y[{:.1}, {:.1}] Z[{:.1}, {:.1}]",
                h.min_x, h.max_x, h.min_y, h.max_y, h.min_z, h.max_z
            );
            println!("    Scale: {}, {}, {}", h.x_scale, h.y_scale, h.z_scale);
            h
        }
        Err(e) => {
            eprintln!("  ✗ FAILED to read header: {}", e);
            std::process::exit(1);
        }
    };
    println!();

    // ── Step 2: Read all points ──
    println!("Step 2: Reading {} points...", header.num_point_records);
    let points = match las::read_points(&path, 0) {
        Ok(p) => {
            println!("  ✓ Read {} points", p.len());
            // Compute basic stats
            let n = p.len();
            let sum_z: f64 = p.iter().map(|(_, _, z)| *z).sum();
            let mean_z = sum_z / n as f64;
            let min_z = p
                .iter()
                .map(|(_, _, z)| *z)
                .fold(f64::INFINITY, |a, b| a.min(b));
            let max_z = p
                .iter()
                .map(|(_, _, z)| *z)
                .fold(f64::NEG_INFINITY, |a, b| a.max(b));
            println!(
                "    Z stats: min={:.3}, max={:.3}, mean={:.3}",
                min_z, max_z, mean_z
            );

            // ── Terrain type detection ──
            let relief = max_z - min_z;
            let extent_x = header.max_x - header.min_x;
            let extent_y = header.max_y - header.min_y;
            let max_extent = extent_x.max(extent_y);
            println!(
                "    Extent: {:.0}m × {:.0}m, relief: {:.1}m",
                extent_x, extent_y, relief
            );

            if relief > 50.0 && max_extent > 500.0 {
                println!();
                println!("  ⚠️  TERRAIN DETECTED (not a stockpile)");
                println!("     The EOM pipeline is designed for stockpile volume");
                println!("     calculation against a flat reference. For natural");
                println!("     terrain, the 'fill volume' is the entire hillside");
                println!("     above the valley floor — not a meaningful number.");
                println!("     Results below are computed correctly but the use");
                println!("     case is wrong. Use Volume Calc with a design surface");
                println!("     (DXF TIN) for terrain volume comparison.");
            } else {
                println!("  ✅ STOCKPILE DETECTED — pipeline is designed for this.");
            }
            println!();

            p
        }
        Err(e) => {
            eprintln!("  ✗ FAILED to read points: {}", e);
            std::process::exit(1);
        }
    };
    println!();

    // ── Step 3: Run the full EOM pipeline ──
    println!("Step 3: Running EOM pipeline (CSF → DEM → volumes)...");
    let input = EomInput {
        point_cloud_path: path.clone(),
        csf_params: metardu_core::mining::csf::CsfParams::default(),
        dem_cell_size: 1.0,  // 1m DEM cells — matches our Python validation
        bench_interval: 2.0, // 2m benches
        max_points: 0,       // 0 = all points
        license_id: String::new(),
        machine_id: String::new(),
        site_id: String::new(),
        signed: false,
        custodian: String::new(),
        baseline_z: None,     // auto-detect via RANSAC histogram mode
        design_surface: None, // flat baseline (stockpile use case)
    };

    let start = std::time::Instant::now();
    let result = run_eom_pipeline(&input, |progress: EomProgress| {
        println!("    → {}", progress.message);
    });
    let elapsed = start.elapsed();

    match result {
        Ok(output) => {
            println!();
            println!(
                "  ✓ EOM pipeline completed in {:.2}s",
                elapsed.as_secs_f64()
            );
            println!();
            println!("─{}", "─".repeat(60));
            println!("RESULTS");
            println!("─{}", "─".repeat(60));
            println!();
            println!("  Points read:       {}", output.points_read);
            println!("  Ground points:     {}", output.ground_points);
            println!("  Non-ground points: {}", output.non_ground_points);
            println!(
                "  DEM:               {}×{} cells @ {}m",
                output.dem.ncols, output.dem.nrows, output.dem.cell_size
            );
            println!("  Audit hash:        {}", &output.audit_hash[..16]);
            println!();
            println!("  ┌─────────────────────────────────────────┐");
            println!(
                "  │ Fill volume:  {:>12.2} m³              │",
                output.volumes.fill_volume
            );
            println!(
                "  │ Cut volume:   {:>12.2} m³              │",
                output.volumes.cut_volume
            );
            println!(
                "  │ Net volume:   {:>12.2} m³              │",
                output.volumes.net_volume
            );
            println!("  └─────────────────────────────────────────┘");
            println!();
            println!(
                "  Fill cells: {}  Cut cells: {}  Cell area: {:.1} m²",
                output.volumes.fill_cells, output.volumes.cut_cells, output.volumes.cell_area
            );
            println!();

            // ── Cross-validation ──
            let expected_cone = std::f64::consts::PI * 20.0_f64.powi(2) * 8.0 / 3.0;
            println!("─{}", "─".repeat(60));
            println!("CROSS-VALIDATION");
            println!("─{}", "─".repeat(60));
            println!();
            println!("  Analytical cone volume: {:.2} m³", expected_cone);
            println!(
                "  Rust pipeline fill:     {:.2} m³",
                output.volumes.fill_volume
            );
            println!(
                "  Rust pipeline net:      {:.2} m³",
                output.volumes.net_volume
            );
            let fill_error =
                (output.volumes.fill_volume - expected_cone).abs() / expected_cone * 100.0;
            let net_error =
                (output.volumes.net_volume - expected_cone).abs() / expected_cone * 100.0;
            println!();
            println!("  Fill vs analytical: {:.2}% error", fill_error);
            println!("  Net vs analytical:  {:.2}% error", net_error);
            println!();

            if fill_error < 5.0 {
                println!("  ✅ PASS — fill volume within 5% of analytical value");
            } else if fill_error < 10.0 {
                println!("  ⚠️  MARGINAL — fill volume within 10% but not 5%");
            } else {
                println!("  ❌ FAIL — fill volume >10% off from analytical value");
            }

            // Bench breakdown
            if !output.volumes.benches.is_empty() {
                println!();
                println!("─{}", "─".repeat(60));
                println!("BENCH BREAKDOWN ({}m intervals)", input.bench_interval);
                println!("─{}", "─".repeat(60));
                println!();
                println!(
                    "  {:>8}  {:>8}  {:>12}  {:>12}  {:>4}",
                    "Z_min", "Z_max", "Fill (m³)", "Cut (m³)", "Cells"
                );
                for bench in &output.volumes.benches {
                    println!(
                        "  {:>8.1}  {:>8.1}  {:>12.2}  {:>12.2}  {:>4}",
                        bench.z_min,
                        bench.z_max,
                        bench.fill_volume,
                        bench.cut_volume,
                        bench.fill_cells + bench.cut_cells
                    );
                }
            }
        }
        Err(e) => {
            eprintln!();
            eprintln!("  ✗ EOM pipeline FAILED: {}", e);
            std::process::exit(1);
        }
    }

    println!();
    println!("═{}", "═".repeat(60));
    println!("Validation complete.");
    println!("═{}", "═".repeat(60));

    // If a second argument is provided (design surface path), run the
    // design surface comparison test.
    if args.len() > 2 {
        run_design_surface_test(&args[1], &args[2]);
    } else if args.len() > 1 && args[1] == "--pit-test" {
        // Convenience: run the pit test with default paths
        run_design_surface_test(
            "test-data/excavated_pit.las",
            "test-data/design_surface.las",
        );
    }
}

/// Run a design-surface comparison test: excavated pit vs flat design.
/// The analytical cut volume = π * r² * h / 3 = 6544.98 m³.
#[allow(dead_code)]
fn run_design_surface_test(excavated_path: &str, _design_path: &str) {
    use metardu_core::mining::eom::DesignSurfaceRef;

    println!();
    println!("═{}", "═".repeat(60));
    println!("DESIGN SURFACE TEST — Pit excavation");
    println!("═{}", "═".repeat(60));
    println!();

    // The design surface is a flat plane at z=100m.
    // We pass it as DesignSurfaceRef::Flat(100.0).
    // In a real workflow, this would be a DXF TIN rasterized to a DEM.
    let input = EomInput {
        point_cloud_path: PathBuf::from(excavated_path),
        csf_params: metardu_core::mining::csf::CsfParams::default(),
        dem_cell_size: 1.0,
        bench_interval: 2.0,
        max_points: 0,
        license_id: String::new(),
        machine_id: String::new(),
        site_id: String::new(),
        signed: false,
        custodian: String::new(),
        baseline_z: None,
        design_surface: Some(DesignSurfaceRef::Flat(100.0)),
    };

    let start = std::time::Instant::now();
    let result = run_eom_pipeline(&input, |p: EomProgress| {
        println!("    → {}", p.message);
    });
    let elapsed = start.elapsed();

    match result {
        Ok(output) => {
            let expected = std::f64::consts::PI * 25.0_f64.powi(2) * 10.0 / 3.0;
            println!();
            println!(
                "  ✓ Design surface test completed in {:.2}s",
                elapsed.as_secs_f64()
            );
            println!();
            println!("  Cut volume:  {:>10.2} m³", output.volumes.cut_volume);
            println!("  Fill volume: {:>10.2} m³", output.volumes.fill_volume);
            println!();
            println!("  Analytical excavation: {:.2} m³", expected);
            println!(
                "  Computed cut:          {:.2} m³",
                output.volumes.cut_volume
            );
            let error = (output.volumes.cut_volume - expected).abs() / expected * 100.0;
            println!("  Error: {:.2}%", error);
            if error < 5.0 {
                println!("  ✅ PASS — cut volume within 5% of analytical");
            } else {
                println!("  ❌ FAIL — cut volume >5% off");
            }
        }
        Err(e) => {
            eprintln!("  ✗ Design surface test FAILED: {}", e);
        }
    }
}
