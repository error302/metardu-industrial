// File format readers for MetaRDU Industrial.
// Phase 1: pure-Rust parsers for LAS, GeoTIFF, Kongsberg .all, Reson .s7k.
// Sprint 6: pure-Rust SSS XTF parser for waterfall viewer.
// Sprint 15: Shapefile reader/writer.

pub mod geotiff;
pub mod kongsberg_all;
pub mod las;
pub mod orthomosaic;
pub mod reson_s7k;
pub mod shapefile;
pub mod sss_xtf;

pub use geotiff::{read_header as read_geotiff_header, sample_profile, GeoTiffHeader};
pub use kongsberg_all::{read_header as read_kongsberg_all_header, AllHeader};
pub use las::{read_header as read_las_header, read_points as read_las_points, LasHeader};
pub use orthomosaic::{read_orthomosaic, Orthomosaic};
pub use reson_s7k::{read_header as read_s7k_header, S7kHeader};
pub use shapefile::{read_shapefile, write_shapefile, Shapefile, ShapefileFeature, Shape, ShapeType};
pub use sss_xtf::{compute_target_height_from_shadow, read_xtf_pings, SssData};
