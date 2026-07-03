// Marine survey module — re-exports from the shared metardu-core crate
// plus local-only modules that haven't been promoted to the shared crate.
//
// The shared marine modules (CUBE, TPU, S-44, S-57, SVP) live in
// crates/metardu-core/src/marine/ and are re-exported here. The dredge
// module is currently app-local because it depends on the desktop app's
// volume infrastructure; promote to metardu-core when the worker binary
// needs it.

#[allow(dead_code)]
pub mod cross_section;
#[allow(dead_code)]
pub mod density_gates;
#[allow(dead_code)]
pub mod dredge;
#[allow(dead_code)]
pub mod tidal_spline;

pub use metardu_core::marine::*;

// Keep the module declarations for backward compatibility with
// any code that references `crate::marine::cube` etc.
#[allow(unused_imports)]
pub mod cube {
    pub use metardu_core::marine::cube::*;
}
#[allow(unused_imports)]
pub mod s44 {
    pub use metardu_core::marine::s44::*;
}
#[allow(unused_imports)]
pub mod s57 {
    pub use metardu_core::marine::s57::*;
}
#[allow(unused_imports)]
pub mod tpu {
    pub use metardu_core::marine::tpu::*;
}
