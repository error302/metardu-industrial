// Marine survey module — re-exports from the shared metardu-core crate.
//
// The actual source files live in crates/metardu-core/src/marine/.
// This module re-exports them so the rest of the app can use them
// without changing import paths.

pub use metardu_core::marine::*;

// Keep the module declarations for backward compatibility with
// any code that references `crate::marine::cube` etc.
pub mod cube {
    pub use metardu_core::marine::cube::*;
}
pub mod s44 {
    pub use metardu_core::marine::s44::*;
}
pub mod s57 {
    pub use metardu_core::marine::s57::*;
}
pub mod tpu {
    pub use metardu_core::marine::tpu::*;
}
