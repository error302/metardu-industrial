// Quality Assurance / Quality Control module — Sprint 12.
//
// This module provides the foundation for calculation verification and
// error propagation across MetaRDU's calculation engine.
//
// Three submodules:
//   - propagation: UncertainValue type + arithmetic for propagating
//     measurement uncertainty through calculations
//   - verify: cross-check framework for running independent calculations
//     and warning when they disagree
//   - range_checks: sanity checks for gross input errors (lat/lon out of
//     range, distances beyond instrument capability, etc.)
//
// Design principle: every critical calculation in MetaRDU should:
//   1. Accept UncertainValue inputs (so uncertainty is never lost)
//   2. Return UncertainValue outputs (so downstream code sees it)
//   3. Be wrapped in verify_calculation() with a secondary method
//   4. Be range-checked at the input boundary
//
// See docs/QA_QC_ANALYSIS.md for the full strategy.

pub mod propagation;
pub mod range_checks;
pub mod verify;

pub use propagation::UncertainValue;
pub use verify::{verify_calculation, VerifiedCalculation};
pub use range_checks::{check_lat_lon, check_elevation, check_distance, check_bearing, RangeCheckResult};
