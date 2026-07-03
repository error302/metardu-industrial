// File format readers for MetaRDU Industrial.
// Phase 1: pure-Rust parsers for LAS, GeoTIFF, Kongsberg .all, Reson .s7k.
// Sprint 6: pure-Rust SSS XTF parser for waterfall viewer.

pub mod geotiff;
pub mod kongsberg_all;
pub mod las;
pub mod reson_s7k;
pub mod sss_xtf;

pub use geotiff::{read_header as read_geotiff_header, sample_profile, GeoTiffHeader};
pub use kongsberg_all::{read_header as read_kongsberg_all_header, AllHeader};
pub use las::{read_header as read_las_header, read_points as read_las_points, LasHeader};
pub use reson_s7k::{read_header as read_s7k_header, S7kHeader};
pub use sss_xtf::{
    compute_target_height_from_shadow, read_xtf_header, read_xtf_pings, sample_index_to_slant_range,
    SssData, SssError, SssPing, XtfHeader,
};
