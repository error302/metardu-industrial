// metardu-verify — standalone chain-of-custody verifier for MetaRDU
// Industrial PDF reports.
//
// This is a free, open-source tool that takes a signed PDF and verifies
// the embedded chain-of-custody record:
//
//   1. Opens the PDF using `lopdf`.
//   2. Extracts the `Keywords` metadata field (which contains the
//      chain-of-custody JSON, sealed by `metardu_core::mining::report`).
//   3. Parses the JSON as a `ChainOfCustody` struct (22 fields).
//   4. Recomputes the SHA-256 `report_hash` from the CoC fields.
//   5. Compares with the embedded hash.
//   6. Prints a human-readable result (or `--json` for machine-readable).
//
// The canonical hash pre-image is the JSON encoding of the CoC struct
// with the `report_hash` field set to the empty string, serialised via
// `serde_json::to_string` (struct declaration order, no whitespace). This
// matches the sealing algorithm in `metardu-core::mining::report::ChainOfCustody::seal`.
//
// As a fallback (for forward-compatibility with future canonicalisation
// schemes), the verifier also tries a "sorted keys, report_hash omitted"
// pre-image and accepts the PDF if either hash matches.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, ValueEnum};
use lopdf::Document;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// 22-field chain-of-custody record. Field names and order MUST match
/// `metardu_core::mining::report::ChainOfCustody` exactly so that the
/// `report_hash` recomputation is byte-for-byte identical.
///
/// This struct is kept as a documentation reference for the CoC schema;
/// the verifier itself works on `serde_json::Value` (with
/// `arbitrary_precision`) so that float representations are preserved
/// exactly across the parse/serialise round-trip.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
struct ChainOfCustody {
    pub custody_id: String,
    pub created_at: u64,
    pub custodian: String,
    pub source_file: String,
    pub source_hash: String,
    pub point_count: u64,
    pub ground_count: u64,
    pub csf_cloth_resolution: f64,
    pub csf_classification_threshold: f64,
    pub csf_iterations: u32,
    pub dem_cell_size: f64,
    pub dem_min_x: f64,
    pub dem_min_y: f64,
    pub dem_max_x: f64,
    pub dem_max_y: f64,
    pub fill_volume: f64,
    pub cut_volume: f64,
    pub net_volume: f64,
    pub license_id: String,
    pub machine_id: String,
    pub site_id: String,
    pub report_hash: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum OutputFormat {
    /// Human-readable text (default).
    Text,
    /// Machine-readable JSON.
    Json,
}

#[derive(Parser)]
#[command(
    name = "metardu-verify",
    version,
    about = "Verify the chain-of-custody of a MetaRDU Industrial PDF report"
)]
struct Cli {
    /// Path to the PDF report to verify.
    pdf: PathBuf,
    /// Output format.
    #[arg(short = 'o', long = "output", value_enum, default_value_t = OutputFormat::Text)]
    output: OutputFormat,
    /// Also accept the "sorted keys, report_hash omitted" canonical form
    /// as a valid hash pre-image. Enabled by default; pass
    /// `--no-sorted-fallback` to require strict library-canonical match.
    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    sorted_fallback: bool,
    /// Disable the sorted-keys fallback (alias for `--sorted-fallback=false`).
    #[arg(long, overrides_with = "sorted_fallback")]
    no_sorted_fallback: bool,
}

impl Cli {
    fn use_sorted_fallback(&self) -> bool {
        if self.no_sorted_fallback {
            false
        } else {
            self.sorted_fallback
        }
    }
}

/// Verifier outcome.
#[derive(Debug, Serialize)]
struct VerifyResult {
    /// Path of the PDF that was verified.
    pdf: String,
    /// True iff the recomputed hash matches the embedded hash.
    valid: bool,
    /// Hash embedded in the PDF's chain-of-custody record.
    embedded_hash: String,
    /// Hash recomputed by this verifier (library-canonical pre-image).
    recomputed_hash: String,
    /// True iff the match came from the sorted-keys fallback pre-image
    /// rather than the library-canonical pre-image.
    matched_via_fallback: bool,
    /// Custody ID of the parsed chain-of-custody record (empty on parse failure).
    custody_id: String,
    /// Number of fields in the parsed chain-of-custody record (0 on parse failure).
    field_count: usize,
    /// Optional error message (empty on success).
    error: String,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = verify_pdf(&cli.pdf, cli.use_sorted_fallback());
    match cli.output {
        OutputFormat::Text => print_text(&result),
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&result).unwrap_or_else(|e| {
                format!(
                    "{{\"error\":\"failed to serialise result: {}\"}}",
                    e.to_string().replace('"', "\\\"")
                )
            });
            println!("{}", json);
        }
    }
    if result.valid {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Run the verification pipeline on the PDF at `path`.
fn verify_pdf(path: &PathBuf, allow_sorted_fallback: bool) -> VerifyResult {
    let pdf_str = path.display().to_string();
    let mut result = VerifyResult {
        pdf: pdf_str.clone(),
        valid: false,
        embedded_hash: String::new(),
        recomputed_hash: String::new(),
        matched_via_fallback: false,
        custody_id: String::new(),
        field_count: 0,
        error: String::new(),
    };

    let document = match Document::load(path) {
        Ok(d) => d,
        Err(e) => {
            result.error = format!("failed to open PDF: {e}");
            return result;
        }
    };

    let keywords = match extract_keywords(&document) {
        Ok(s) => s,
        Err(e) => {
            result.error = format!("failed to extract Keywords metadata: {e}");
            return result;
        }
    };

    // Parse the chain-of-custody JSON as a `serde_json::Value` (with the
    // `arbitrary_precision` and `preserve_order` features enabled at the
    // crate level). `arbitrary_precision` preserves the original textual
    // representation of every number — without it, serde_json's float
    // parser can yield a different f64 than the producer wrote, breaking
    // the byte-exact hash comparison. `preserve_order` keeps the field
    // order from the JSON, which matches the producer's struct declaration
    // order.
    let mut value: serde_json::Value = match serde_json::from_str(&keywords) {
        Ok(v) => v,
        Err(e) => {
            result.error = format!("failed to parse chain-of-custody JSON: {e}");
            return result;
        }
    };

    // Record summary fields for the result.
    if let serde_json::Value::Object(map) = &value {
        result.field_count = map.len();
    }
    result.custody_id = value
        .get("custody_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // The embedded hash is what the producer sealed.
    let embedded_hash = value
        .get("report_hash")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    result.embedded_hash = embedded_hash.clone();

    // --- Library-canonical pre-image ----------------------------------
    // The producer's `seal()` sets `report_hash = ""` and serialises the
    // struct with `serde_json::to_string` (declaration order, no
    // whitespace). Reproduce that exactly by setting the `report_hash`
    // field to "" in the parsed Value and re-serialising. With
    // `arbitrary_precision` + `preserve_order`, this is byte-for-byte
    // identical to what the producer computed.
    if let serde_json::Value::Object(ref mut map) = value {
        map.insert(
            "report_hash".to_string(),
            serde_json::Value::String(String::new()),
        );
    }
    let canonical_json = serde_json::to_string(&value).unwrap_or_default();
    let canonical_hash = sha256_hex(canonical_json.as_bytes());
    result.recomputed_hash = canonical_hash.clone();

    if constant_time_eq(canonical_hash.as_bytes(), embedded_hash.as_bytes()) {
        result.valid = true;
        result.matched_via_fallback = false;
        return result;
    }

    // --- Sorted-keys fallback (excludes report_hash entirely) ---------
    // Some hypothetical future producer might seal the hash over a
    // canonical form with alphabetically-sorted keys and `report_hash`
    // omitted entirely. We accept that form too, for forward
    // compatibility. (The current `metardu-core` producer does NOT use
    // this form, so the fallback only matters for cross-version
    // verification.)
    if allow_sorted_fallback {
        if let serde_json::Value::Object(mut map) = value.clone() {
            map.remove("report_hash");
            // Convert to a BTreeMap so keys come out alphabetically
            // sorted when serialised, regardless of the `preserve_order`
            // feature on serde_json::Map.
            let btree: std::collections::BTreeMap<String, serde_json::Value> =
                map.into_iter().collect();
            let sorted_json = serde_json::to_string(&btree).unwrap_or_default();
            let sorted_hash = sha256_hex(sorted_json.as_bytes());
            if constant_time_eq(sorted_hash.as_bytes(), embedded_hash.as_bytes()) {
                result.valid = true;
                result.matched_via_fallback = true;
                return result;
            }
        }
    }

    result.error = format!(
        "hash mismatch: embedded={} recomputed={}",
        embedded_hash, canonical_hash
    );
    result
}

/// Extract the `Keywords` string from the PDF's `/Info` dictionary.
///
/// Steps:
///   1. `document.trailer.get(b"Info")` → reference object.
///   2. `.as_reference()` → `ObjectId`.
///   3. `document.get_object(id)` → the Info dict.
///   4. `dict.get(b"Keywords")` → string object.
///   5. `.as_str()` (raw bytes) or `.as_string()` (Cow<str>).
fn extract_keywords(document: &Document) -> Result<String, String> {
    let info_ref = document
        .trailer
        .get(b"Info")
        .map_err(|e| format!("trailer has no /Info entry: {e}"))?;
    let info_id = info_ref
        .as_reference()
        .map_err(|e| format!("/Info is not an indirect reference: {e}"))?;
    let info_obj = document
        .get_object(info_id)
        .map_err(|e| format!("could not fetch /Info object {info_id:?}: {e}"))?;
    let info_dict = info_obj
        .as_dict()
        .map_err(|e| format!("/Info is not a dictionary: {e}"))?;
    let keywords_obj = info_dict
        .get(b"Keywords")
        .map_err(|e| format!("/Info has no /Keywords entry: {e}"))?;
    // Prefer `as_string` (returns Cow<str> with UTF-8 lossy conversion)
    // since printpdf writes Keywords as a literal PDF string.
    if let Ok(cow) = keywords_obj.as_string() {
        return Ok(cow.into_owned());
    }
    if let Ok(bytes) = keywords_obj.as_str() {
        return Ok(String::from_utf8_lossy(bytes).into_owned());
    }
    Err("/Keywords is not a string".to_string())
}

/// Compute the SHA-256 of a byte slice and return it as a lowercase hex
/// string.
fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    digest.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Constant-time comparison to prevent timing side-channels on the hash
/// comparison.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Print the verification result in a human-readable format.
fn print_text(result: &VerifyResult) {
    println!("MetaRDU PDF Chain-of-Custody Verifier");
    println!("======================================");
    println!("PDF           : {}", result.pdf);
    println!("Custody ID    : {}", result.custody_id);
    println!("CoC fields    : {}", result.field_count);
    println!("Embedded hash : {}", result.embedded_hash);
    println!("Recomputed    : {}", result.recomputed_hash);
    if result.valid {
        if result.matched_via_fallback {
            println!("Status        : VALID (matched via sorted-keys fallback)");
        } else {
            println!("Status        : VALID");
        }
    } else {
        println!("Status        : INVALID");
        if !result.error.is_empty() {
            println!("Reason        : {}", result.error);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256_hex_known_value() {
        // SHA-256 of "hello world"
        let h = sha256_hex(b"hello world");
        assert_eq!(
            h,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_constant_time_eq_basic() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"abcd"));
        assert!(constant_time_eq(b"", b""));
    }

    #[test]
    fn test_coc_has_22_fields() {
        let coc = ChainOfCustody {
            custody_id: "CUSTODY-001".to_string(),
            created_at: 1_700_000_000,
            custodian: "J. Surveyor".to_string(),
            source_file: "/data/survey_001.las".to_string(),
            source_hash: "abcdef0123456789".to_string(),
            point_count: 12_345,
            ground_count: 11_000,
            csf_cloth_resolution: 0.5,
            csf_classification_threshold: 0.5,
            csf_iterations: 500,
            dem_cell_size: 1.0,
            dem_min_x: 500_000.0,
            dem_min_y: 4_000_000.0,
            dem_max_x: 500_100.0,
            dem_max_y: 4_000_100.0,
            fill_volume: 1234.5,
            cut_volume: 567.8,
            net_volume: 666.7,
            license_id: "LIC-001".to_string(),
            machine_id: "MACHINE-ABC".to_string(),
            site_id: "SITE-001".to_string(),
            report_hash: String::new(),
        };
        let json = serde_json::to_value(&coc).unwrap();
        let obj = json.as_object().unwrap();
        assert_eq!(obj.len(), 22, "ChainOfCustody must have exactly 22 fields");
        assert!(obj.contains_key("report_hash"));
    }

    #[test]
    fn test_canonical_hash_is_deterministic() {
        let mut coc = ChainOfCustody {
            custody_id: "CUSTODY-001".to_string(),
            created_at: 1_700_000_000,
            custodian: "JSurveyor".to_string(),
            source_file: "/data/survey_001.las".to_string(),
            source_hash: "abcdef0123456789".to_string(),
            point_count: 12_345,
            ground_count: 11_000,
            csf_cloth_resolution: 0.5,
            csf_classification_threshold: 0.5,
            csf_iterations: 500,
            dem_cell_size: 1.0,
            dem_min_x: 500_000.0,
            dem_min_y: 4_000_000.0,
            dem_max_x: 500_100.0,
            dem_max_y: 4_000_100.0,
            fill_volume: 1234.5,
            cut_volume: 567.8,
            net_volume: 666.7,
            license_id: "LIC-001".to_string(),
            machine_id: "MACHINE-ABC".to_string(),
            site_id: "SITE-001".to_string(),
            report_hash: String::new(),
        };
        coc.report_hash.clear();
        let json1 = serde_json::to_string(&coc).unwrap();
        let json2 = serde_json::to_string(&coc).unwrap();
        assert_eq!(json1, json2);
        // No structural whitespace in canonical form (no spaces after
        // `{`, `,`, `:`, or before `}`). String values may legitimately
        // contain spaces — we use a space-free custodian here to keep
        // the check simple.
        assert!(!json1.contains(' '));
        assert!(!json1.contains("\n"));
        // report_hash field is present and empty.
        assert!(json1.contains("\"report_hash\":\"\""));
    }
}
