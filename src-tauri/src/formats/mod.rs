// File format readers for MetaRDU Industrial.
// Phase 0: pure-Rust parsers for LAS and GeoTIFF. More formats to follow:
//   - Kongsberg .all (multibeam datagram reader)
//   - Reson .s7k
//   - R2Sonic .bsf

pub mod geotiff;
pub mod las;

pub use geotiff::{read_header as read_geotiff_header, GeoTiffError, GeoTiffHeader};
pub use las::{read_header as read_las_header, LasHeader};
