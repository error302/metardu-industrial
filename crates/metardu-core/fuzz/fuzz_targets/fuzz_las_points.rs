//! Fuzz target for the LAS point reader.
//!
//! Writes the fuzz input to a temp file, then calls `read_points()`
//! with max_points=1000 (to bound runtime). Any panic or crash is a
//! bug — a malicious LAS file with a crafted header must not OOM or
//! panic the reader.

#![no_main]

use libfuzzer_sys::fuzz_target;
use metardu_core::mining::las::read_points;
use std::io::Write;

fuzz_target!(|data: &[u8]| {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    tmp.as_file().write_all(data).unwrap();
    // max_points=1000 bounds the runtime — the fuzzer would be too
    // slow if it tried to read millions of points per iteration.
    let _ = read_points(tmp.path(), 1000);
});
