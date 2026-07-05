//! Fuzz target for the LAS header parser.
//!
//! Writes the fuzz input to a temp file, then calls `read_header()`.
//! Any panic or crash is a bug — the parser must handle arbitrary
//! bytes gracefully and return an error, not panic.

#![no_main]

use libfuzzer_sys::fuzz_target;
use metardu_core::mining::las::read_header;
use std::io::Write;

fuzz_target!(|data: &[u8]| {
    // Write the fuzz input to a temp file and try to parse it as a
    // LAS header. We don't care if it succeeds or fails — we only
    // care that it doesn't panic or hang.
    let tmp = tempfile::NamedTempFile::new().unwrap();
    tmp.as_file().write_all(data).unwrap();
    let _ = read_header(tmp.path());
});
