// File format readers for MetaRDU Industrial.
// Phase 0: pure-Rust LAS header parser. More formats to follow:
//   - GeoTIFF (via tiff crate + custom GeoKey directory reader)
//   - Kongsberg .all (multibeam datagram reader)
//   - Reson .s7k
//   - R2Sonic .bsf

pub mod las;

pub use las::{read_header as read_las_header, LasError, LasHeader};
