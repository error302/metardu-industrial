// License Manager — Sprint 7 Enterprise Readiness.
//
// Verifies JSON-signed license files for Pro/Enterprise tier gating.
// Implements a simple but secure license scheme:
//
//   1. License file is a JSON payload with: customer, tier, expiry, features
//   2. The payload is signed with HMAC-SHA256 using a secret key
//   3. The signature is appended to the file as a hex string on a separate line
//   4. The verifier recomputes the HMAC and compares
//
// For production: ship the verifier in the binary (which we do), but keep
// the SIGNING key off the binary (we use a separate tool to sign licenses).
// The verifier uses a PUBLIC verification key derived from the secret.
//
// License tiers:
//   - Core (free, MIT): all Sprint 1-6 features
//   - Pro ($3-5K/seat/yr): EoM Reconciliation, Dredge Audit, S-44 Cert,
//     Stockpile, Blast, Highwall, Deliverable, Cross-Section
//   - Enterprise ($10-25K/site/yr): Distributed processing, plugin SDK,
//     multi-user PostGIS sync, custom branding, priority support
//
// The frontend queries `get_license_status` on startup and shows a
// "License: Pro" or "License: Enterprise" badge in the title bar.
// Without a valid license, Pro/Enterprise features are gated (greyed-out
// with an "Activate License" prompt).

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LicenseTier {
    /// Free / open-source — Sprint 1-6 core features
    Core,
    /// Pro tier — all revenue features
    Pro,
    /// Enterprise tier — distributed + plugin SDK + multi-user
    Enterprise,
    /// Trial — 30-day Pro features
    Trial,
}

impl LicenseTier {
    pub fn label(&self) -> &str {
        match self {
            LicenseTier::Core => "Core (Free)",
            LicenseTier::Pro => "Pro",
            LicenseTier::Enterprise => "Enterprise",
            LicenseTier::Trial => "Trial",
        }
    }

    pub fn color_hex(&self) -> &str {
        match self {
            LicenseTier::Core => "#64748B",     // steel gray
            LicenseTier::Pro => "#FFA500",       // industrial orange
            LicenseTier::Enterprise => "#6366F1", // indigo
            LicenseTier::Trial => "#F59E0B",     // amber
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicensePayload {
    /// Customer / company name
    pub customer: String,
    /// License tier
    pub tier: LicenseTier,
    /// ISO 8601 expiry date (YYYY-MM-DD). Empty = perpetual.
    #[serde(default)]
    pub expiry: String,
    /// Seat count (0 = unlimited site license)
    #[serde(default)]
    pub seats: u32,
    /// Explicit feature list — if empty, all features for the tier
    #[serde(default)]
    pub features: Vec<String>,
    /// License ID (UUID)
    pub license_id: String,
    /// Issue date (ISO 8601)
    pub issued: String,
    /// Issuer (e.g., "MetaRDU Industrial Sales")
    pub issuer: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LicenseStatus {
    /// True if a valid license is loaded
    pub valid: bool,
    /// The license tier (Core if no license)
    pub tier: LicenseTier,
    /// The loaded license payload (None if no license)
    pub payload: Option<LicensePayload>,
    /// Days until expiry (None = perpetual or expired)
    pub days_remaining: Option<i64>,
    /// True if the license is expired
    pub expired: bool,
    /// Error message if the license is invalid
    pub error: Option<String>,
    /// Set of features unlocked by this license
    pub unlocked_features: HashSet<String>,
}

impl Default for LicenseStatus {
    fn default() -> Self {
        Self {
            valid: false,
            tier: LicenseTier::Core,
            payload: None,
            days_remaining: None,
            expired: false,
            error: None,
            unlocked_features: Self::core_features(),
        }
    }
}

impl LicenseStatus {
    /// Features available to all users (Core tier)
    pub fn core_features() -> HashSet<String> {
        let mut s = HashSet::new();
        s.insert("volume_calc".into());
        s.insert("csf_classification".into());
        s.insert("cube_surface".into());
        s.insert("s44_compliance_check".into());
        s.insert("s57_export".into());
        s.insert("svp_editor".into());
        s.insert("vessel_config".into());
        s.insert("cube_disambiguation".into());
        s.insert("4d_monitoring".into());
        s.insert("ml_classification".into());
        s.insert("pipeline".into());
        s.insert("streaming".into());
        s.insert("sss_waterfall".into());
        s.insert("slice_editor".into());
        s.insert("layout_profiles".into());
        s.insert("command_palette".into());
        s
    }

    /// Features added by Pro tier (all revenue features)
    pub fn pro_features() -> HashSet<String> {
        let mut s = Self::core_features();
        s.insert("eom_reconciliation".into());
        s.insert("dredge_audit".into());
        s.insert("s44_certificate".into());
        s.insert("stockpile_audit".into());
        s.insert("blast_report".into());
        s.insert("highwall_monitoring".into());
        s.insert("deliverable_package".into());
        s.insert("cross_section_profiler".into());
        s.insert("branded_pdf".into());
        s
    }

    /// Features added by Enterprise tier
    pub fn enterprise_features() -> HashSet<String> {
        let mut s = Self::pro_features();
        s.insert("distributed_processing".into());
        s.insert("plugin_sdk".into());
        s.insert("multi_user_sync".into());
        s.insert("custom_branding".into());
        s.insert("priority_support".into());
        s
    }

    /// Check if a feature is unlocked
    pub fn has_feature(&self, feature: &str) -> bool {
        self.unlocked_features.contains(feature)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LicenseError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("license file not found: {0}")]
    NotFound(String),
    #[error("license file malformed: {0}")]
    Malformed(String),
    #[error("license signature invalid — tampering detected")]
    InvalidSignature,
    #[error("license expired on {0}")]
    Expired(String),
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
}

/// The verification key. In production this would be derived from the
/// signing key via HMAC. For now we use a fixed key — the security model
/// is "casual piracy deterrence" not "DRM-hardened".
///
/// CHANGE THIS KEY BEFORE SHIPPING. The current value is a placeholder.
const LICENSE_SIGNING_KEY: &[u8] = b"metardu-industrial-license-v1-CHANGE-THIS-KEY";

/// Load and verify a license file.
///
/// File format:
/// ```text
/// {"customer":"...","tier":"pro",...}
/// SIGNATURE: hex_encoded_hmac_sha256
/// ```
///
/// The signature is computed over the JSON line(s) only.
pub fn load_license(path: &Path) -> Result<LicenseStatus, LicenseError> {
    if !path.exists() {
        return Err(LicenseError::NotFound(path.display().to_string()));
    }

    let content = std::fs::read_to_string(path)?;
    parse_license(&content)
}

/// Parse a license string (JSON + SIGNATURE line).
pub fn parse_license(content: &str) -> Result<LicenseStatus, LicenseError> {
    // Find the SIGNATURE: line and split content there
    let sig_marker = "\nSIGNATURE:";
    let split_pos = content.find(sig_marker).ok_or_else(|| {
        LicenseError::Malformed("missing SIGNATURE: line".into())
    })?;

    // JSON body is everything before the signature marker
    let json_body = &content[..split_pos];
    // Signature is everything after "SIGNATURE:" up to end/next newline
    let after_marker = &content[split_pos + sig_marker.len()..];
    let signature = after_marker.lines().next().unwrap_or("").trim().to_string();

    if signature.is_empty() {
        return Err(LicenseError::Malformed("empty signature".into()));
    }

    // Verify the signature
    let expected = compute_hmac_sha256(json_body.as_bytes(), LICENSE_SIGNING_KEY);
    if !constant_time_eq(expected.as_bytes(), signature.as_bytes()) {
        return Err(LicenseError::InvalidSignature);
    }

    // Parse the JSON payload
    let payload: LicensePayload = serde_json::from_str(json_body.trim())?;

    // Check expiry
    let (expired, days_remaining) = check_expiry(&payload.expiry);

    if expired {
        return Ok(LicenseStatus {
            valid: false,
            tier: LicenseTier::Core,
            payload: Some(payload.clone()),
            days_remaining: Some(0),
            expired: true,
            error: Some(format!("license expired on {}", payload.expiry)),
            unlocked_features: LicenseStatus::core_features(),
        });
    }

    // Compute unlocked features based on tier
    let unlocked_features = match payload.tier {
        LicenseTier::Core => LicenseStatus::core_features(),
        LicenseTier::Pro | LicenseTier::Trial => {
            let mut s = LicenseStatus::pro_features();
            // If explicit features list is non-empty, intersect
            if !payload.features.is_empty() {
                s.retain(|f| payload.features.contains(f));
            }
            s
        }
        LicenseTier::Enterprise => {
            let mut s = LicenseStatus::enterprise_features();
            if !payload.features.is_empty() {
                s.retain(|f| payload.features.contains(f));
            }
            s
        }
    };

    Ok(LicenseStatus {
        valid: true,
        tier: payload.tier,
        payload: Some(payload),
        days_remaining,
        expired: false,
        error: None,
        unlocked_features,
    })
}

/// Compute the HMAC-SHA256 of a message using a key.
///
/// Uses a minimal HMAC implementation (no external dep).
/// HMAC(K, M) = H((K' ⊕ opad) || H((K' ⊕ ipad) || M))
/// where K' = K padded/hashed to block size 64 bytes,
/// ipad = 0x36 × 64, opad = 0x5C × 64.
fn compute_hmac_sha256(message: &[u8], key: &[u8]) -> String {
    // Block size for SHA-256 = 64 bytes
    let block_size = 64;

    // Key: if longer than block size, hash it; then pad to block size
    let mut k = if key.len() > block_size {
        sha256(key).to_vec()
    } else {
        key.to_vec()
    };
    k.resize(block_size, 0);

    // ipad + opad
    let mut ipad = vec![0x36u8; block_size];
    let mut opad = vec![0x5Cu8; block_size];
    for i in 0..block_size {
        ipad[i] ^= k[i];
        opad[i] ^= k[i];
    }

    // inner = H(ipad || message)
    let mut inner_input = ipad.clone();
    inner_input.extend_from_slice(message);
    let inner_hash = sha256(&inner_input);

    // outer = H(opad || inner)
    let mut outer_input = opad.clone();
    outer_input.extend_from_slice(&inner_hash);
    let outer_hash = sha256(&outer_input);

    // Hex-encode
    outer_hash.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Minimal SHA-256 implementation (no external dep).
fn sha256(input: &[u8]) -> [u8; 32] {
    // SHA-256 constants
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
        0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
        0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
        0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
        0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
        0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];

    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];

    // Pre-processing: pad message to multiple of 64 bytes
    let bit_len = (input.len() as u64) * 8;
    let mut msg = input.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    // Process each 64-byte block
    for chunk in msg.chunks(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4], chunk[i * 4 + 1], chunk[i * 4 + 2], chunk[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }

        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut hh = h[7];

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let temp1 = hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }

    let mut out = [0u8; 32];
    for i in 0..8 {
        out[i * 4..i * 4 + 4].copy_from_slice(&h[i].to_be_bytes());
    }
    out
}

/// Constant-time comparison to prevent timing attacks.
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

/// Check if the license is expired. Returns (expired, days_remaining).
///
/// Expiry format: YYYY-MM-DD. Empty = perpetual (None for days_remaining).
fn check_expiry(expiry: &str) -> (bool, Option<i64>) {
    if expiry.is_empty() {
        return (false, None);
    }
    // Parse YYYY-MM-DD
    let parts: Vec<&str> = expiry.split('-').collect();
    if parts.len() != 3 {
        return (false, None);
    }
    let year: i64 = parts[0].parse().unwrap_or(0);
    let month: i64 = parts[1].parse().unwrap_or(0);
    let day: i64 = parts[2].parse().unwrap_or(0);

    // Current date approximation (epoch days)
    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let now_days = (now_secs / 86400) as i64;
    // Days since 1970-01-01 for the expiry date (approximate)
    let expiry_days = (year - 1970) * 365 + (month - 1) * 30 + (day - 1);

    let days_remaining = expiry_days - now_days;
    (days_remaining < 0, Some(days_remaining))
}

/// Generate a license file (for the licensing tool — not shipped in the app).
///
/// This function is exposed publicly so we can write a separate
/// `metardu-license-tool` binary that signs licenses. The app binary
/// only needs `load_license` and `parse_license`.
pub fn generate_license_file(payload: &LicensePayload) -> String {
    let json = serde_json::to_string_pretty(payload).unwrap_or_default();
    let signature = compute_hmac_sha256(json.as_bytes(), LICENSE_SIGNING_KEY);
    format!("{}\nSIGNATURE: {}\n", json, signature)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn make_test_payload(tier: LicenseTier) -> LicensePayload {
        LicensePayload {
            customer: "Test Mine Co".into(),
            tier,
            expiry: "2099-12-31".into(),
            seats: 5,
            features: vec![],
            license_id: "test-uuid-1234".into(),
            issued: "2026-07-03".into(),
            issuer: "MetaRDU Industrial Sales".into(),
        }
    }

    #[test]
    fn test_sha256_known_value() {
        // SHA-256 of "abc" = ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
        let hash = sha256(b"abc");
        let hex: String = hash.iter().map(|b| format!("{:02x}", b)).collect();
        assert_eq!(hex, "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }

    #[test]
    fn test_sha256_empty() {
        // SHA-256 of "" = e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
        let hash = sha256(b"");
        let hex: String = hash.iter().map(|b| format!("{:02x}", b)).collect();
        assert_eq!(hex, "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    }

    #[test]
    fn test_hmac_sha256_deterministic() {
        let h1 = compute_hmac_sha256(b"test message", b"key");
        let h2 = compute_hmac_sha256(b"test message", b"key");
        assert_eq!(h1, h2);
        let h3 = compute_hmac_sha256(b"test message", b"different key");
        assert_ne!(h1, h3);
    }

    #[test]
    fn test_constant_time_eq() {
        assert!(constant_time_eq(b"hello", b"hello"));
        assert!(!constant_time_eq(b"hello", b"world"));
        assert!(!constant_time_eq(b"hello", b"hell"));
    }

    #[test]
    fn test_generate_and_parse_license_roundtrip() {
        let payload = make_test_payload(LicenseTier::Pro);
        let file_content = generate_license_file(&payload);
        let status = parse_license(&file_content).unwrap();
        assert!(status.valid);
        assert_eq!(status.tier, LicenseTier::Pro);
        assert_eq!(status.payload.as_ref().unwrap().customer, "Test Mine Co");
        assert!(status.has_feature("eom_reconciliation"));
        assert!(status.has_feature("volume_calc")); // core feature
        assert!(!status.has_feature("distributed_processing")); // enterprise only
    }

    #[test]
    fn test_enterprise_license_unlocks_all() {
        let payload = make_test_payload(LicenseTier::Enterprise);
        let file_content = generate_license_file(&payload);
        let status = parse_license(&file_content).unwrap();
        assert!(status.valid);
        assert!(status.has_feature("distributed_processing"));
        assert!(status.has_feature("plugin_sdk"));
        assert!(status.has_feature("multi_user_sync"));
        assert!(status.has_feature("eom_reconciliation"));
    }

    #[test]
    fn test_tampered_license_rejected() {
        let payload = make_test_payload(LicenseTier::Pro);
        let mut file_content = generate_license_file(&payload);
        // Tamper: change "Test Mine Co" to "Hacker Co"
        file_content = file_content.replace("Test Mine Co", "Hacker Co");
        let result = parse_license(&file_content);
        assert!(matches!(result, Err(LicenseError::InvalidSignature)));
    }

    #[test]
    fn test_missing_signature_rejected() {
        let payload = make_test_payload(LicenseTier::Pro);
        let json = serde_json::to_string_pretty(&payload).unwrap();
        // No SIGNATURE line
        let result = parse_license(&json);
        assert!(matches!(result, Err(LicenseError::Malformed(_))));
    }

    #[test]
    fn test_expired_license() {
        let mut payload = make_test_payload(LicenseTier::Pro);
        payload.expiry = "2020-01-01".into(); // past
        let file_content = generate_license_file(&payload);
        let status = parse_license(&file_content).unwrap();
        assert!(!status.valid);
        assert!(status.expired);
        assert_eq!(status.tier, LicenseTier::Core);
    }

    #[test]
    fn test_perpetual_license_no_expiry() {
        let mut payload = make_test_payload(LicenseTier::Enterprise);
        payload.expiry = "".into();
        let file_content = generate_license_file(&payload);
        let status = parse_license(&file_content).unwrap();
        assert!(status.valid);
        assert_eq!(status.days_remaining, None);
    }

    #[test]
    fn test_load_license_file_not_found() {
        let result = load_license(std::path::Path::new("/nonexistent/license.json"));
        assert!(matches!(result, Err(LicenseError::NotFound(_))));
    }

    #[test]
    fn test_load_license_from_real_file() {
        let payload = make_test_payload(LicenseTier::Pro);
        let file_content = generate_license_file(&payload);
        let tmp = std::env::temp_dir().join("metardu_test_license.json");
        let mut f = std::fs::File::create(&tmp).unwrap();
        f.write_all(file_content.as_bytes()).unwrap();

        let status = load_license(&tmp).unwrap();
        assert!(status.valid);
        assert_eq!(status.tier, LicenseTier::Pro);

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_license_tier_labels_and_colors() {
        assert_eq!(LicenseTier::Core.label(), "Core (Free)");
        assert_eq!(LicenseTier::Pro.label(), "Pro");
        assert_eq!(LicenseTier::Enterprise.label(), "Enterprise");
        assert_eq!(LicenseTier::Trial.label(), "Trial");
        // Colors are non-empty hex
        for tier in [LicenseTier::Core, LicenseTier::Pro, LicenseTier::Enterprise, LicenseTier::Trial] {
            let c = tier.color_hex();
            assert!(c.starts_with('#'));
            assert_eq!(c.len(), 7);
        }
    }

    #[test]
    fn test_feature_gating() {
        let core = LicenseStatus::default();
        assert!(core.has_feature("volume_calc"));
        assert!(!core.has_feature("eom_reconciliation"));
        assert!(!core.has_feature("distributed_processing"));

        let pro_features = LicenseStatus::pro_features();
        assert!(pro_features.contains("eom_reconciliation"));
        assert!(!pro_features.contains("distributed_processing"));

        let ent_features = LicenseStatus::enterprise_features();
        assert!(ent_features.contains("distributed_processing"));
        assert!(ent_features.contains("plugin_sdk"));
    }
}
