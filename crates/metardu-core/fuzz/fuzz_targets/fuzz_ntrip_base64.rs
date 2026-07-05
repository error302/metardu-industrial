//! Fuzz target for the NTRIP base64 encoder.
//!
//! Feeds arbitrary strings to `base64_encode()`. The encoder must
//! never panic regardless of input (empty, non-ASCII, very long).

#![no_main]

use libfuzzer_sys::fuzz_target;
use metardu_core::ntrip::base64_encode;

fuzz_target!(|data: &[u8]| {
    // Try to interpret as UTF-8; if it fails, just use lossy conversion.
    let s = String::from_utf8_lossy(data);
    let encoded = base64_encode(&s);
    // The encoded output must always be valid base64 (length divisible
    // by 4, only valid chars). We don't decode here — just ensure no
    // panic.
    let _ = encoded;
});
