//! Fuzz target for the RTCM v3 message parser.
//!
//! Feeds arbitrary bytes to `parse_rtcm_message()`. The parser must
//! handle any input gracefully — returning Ok(None) for incomplete
//! data, Err for CRC failures, or Ok(Some(...)) for valid frames.
//! Any panic is a bug.

#![no_main]

use libfuzzer_sys::fuzz_target;
use metardu_core::ntrip::parse_rtcm_message;

fuzz_target!(|data: &[u8]| {
    let _ = parse_rtcm_message(data);
});
