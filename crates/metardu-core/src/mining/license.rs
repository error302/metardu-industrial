// RSA-2048 node-locked license verification.
//
// This module implements the offline licensing scheme used by the
// metardu-industrial desktop app:
//
//   1. On first launch, the app computes a `MachineFingerprint` (machine_id
//      + site_id) and sends it to the issuing authority.
//   2. The authority signs a `LicenseClaims` JSON blob with its RSA-2048
//      private key and returns a `LicenseFile` (claims + signature).
//   3. The app stores the license file, and on every launch calls
//      `check_status(license, pub_key, machine_id, site_id, trial_quota)`
//      to determine whether to run in Trial, Active, Invalid, Exhausted,
//      or Expired mode.
//
// The signature scheme is RSASSA-PKCS1-v1_5 over SHA-256 (RFC 8017 §8.2),
// implemented via `rsa::pkcs1v15::SigningKey` / `VerifyingKey`. Keys are
// exchanged as PKCS#8 PEM strings.

use std::path::Path;

use rand::rngs::OsRng;
use rsa::{
    pkcs1v15::{Signature, SigningKey, VerifyingKey},
    pkcs8::{DecodePrivateKey, DecodePublicKey, EncodePrivateKey, EncodePublicKey, LineEnding},
    RsaPrivateKey, RsaPublicKey,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Public re-export of the RSA public key type for downstream consumers.
pub type RsaPubKey = RsaPublicKey;
/// Public re-export of the RSA private key type for downstream consumers.
pub type RsaPrivKey = RsaPrivateKey;

/// RSA key size in bits. 2048 is the minimum recommended for RSASSA-PKCS1-v1_5
/// with SHA-256 today.
pub const RSA_KEY_BITS: usize = 2048;

/// Quota of reports allowed during a trial (no license file present).
pub const DEFAULT_TRIAL_QUOTA: u32 = 5;

/// Claims embedded in a license file. These are the fields the issuing
/// authority vouches for; tampering with any of them invalidates the
/// signature.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseClaims {
    /// Unique identifier for this license (UUID or similar).
    pub license_id: String,
    /// Human-readable customer / organisation name.
    pub customer: String,
    /// Machine fingerprint the license is locked to. Must match the
    /// local machine's `MachineFingerprint::machine_id`.
    pub machine_id: String,
    /// Optional site identifier (e.g. mine-site code).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub site_id: Option<String>,
    /// Unix timestamp (seconds) when the license was issued.
    pub issued_at: u64,
    /// Optional Unix timestamp when the license expires.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<u64>,
    /// Optional count of remaining reports the licensee is allowed to
    /// generate. Decremented locally per report by `ReportCounter`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reports_remaining: Option<u32>,
}

/// A signed license file — claims plus the RSA-PKCS1v15-SHA256 signature
/// over the canonical JSON encoding of those claims.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseFile {
    /// The signed claims.
    pub claims: LicenseClaims,
    /// Raw RSA signature bytes (big-endian).
    #[serde(with = "serde_bytes_compat")]
    pub signature: Vec<u8>,
    /// Signature algorithm identifier (always "RS256" for this module).
    pub algorithm: String,
}

/// A locally-computed machine fingerprint used for node-locking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineFingerprint {
    /// Stable hash of host OS + hostname + MAC address(es).
    pub machine_id: String,
    /// Optional site identifier (operator-provided).
    pub site_id: String,
}

/// Result of `check_status`: tells the application whether to grant
/// Trial, Active, or no access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LicenseStatus {
    /// No license file present — running under the trial quota.
    Trial,
    /// License file is present, signature is valid, not expired, and the
    /// machine fingerprint matches.
    Active,
    /// License signature is invalid, claims are malformed, or the machine
    /// fingerprint does not match.
    Invalid,
    /// License is valid but `reports_remaining` has reached zero.
    Exhausted,
    /// License is valid but `expires_at` is in the past.
    Expired,
}

#[derive(Debug, thiserror::Error)]
pub enum LicenseError {
    #[error("RSA error: {0}")]
    Rsa(#[from] rsa::Error),
    #[error("PEM encoding error: {0}")]
    Pem(#[from] rsa::pkcs8::Error),
    #[error("SPKI error: {0}")]
    Spki(#[from] rsa::pkcs8::spki::Error),
    #[error("signature verification failed")]
    InvalidSignature,
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("base64 error: {0}")]
    Base64(#[from] base64::DecodeError),
}

// --- Serde helper for the raw signature bytes ---------------------------

/// Serialise `Vec<u8>` as a base64 string for human-readable formats.
mod serde_bytes_compat {
    use base64::Engine;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(v: &Vec<u8>, s: S) -> Result<S::Ok, S::Error> {
        base64::engine::general_purpose::STANDARD.encode(v).serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        let s = String::deserialize(d)?;
        base64::engine::general_purpose::STANDARD
            .decode(s.as_bytes())
            .map_err(serde::de::Error::custom)
    }
}

/// Generate a fresh RSA-2048 keypair using the OS RNG.
pub fn generate_license_keypair() -> Result<(RsaPrivateKey, RsaPublicKey), LicenseError> {
    let mut rng = OsRng;
    let priv_key = RsaPrivateKey::new(&mut rng, RSA_KEY_BITS)?;
    let pub_key = RsaPublicKey::from(&priv_key);
    Ok((priv_key, pub_key))
}

/// Canonicalise `claims` to a deterministic JSON byte vector for signing.
///
/// The JSON is sorted by key (via `serde_json`'s BTreeMap-backed `Value`)
/// so that the issuing authority and the verifier agree on the byte layout
/// regardless of struct field declaration order.
pub fn canonical_claims_bytes(claims: &LicenseClaims) -> Result<Vec<u8>, LicenseError> {
    let value = serde_json::to_value(claims)?;
    let mut buf = serde_json::to_vec(&value)?;
    // `serde_json::to_vec` already produces deterministic output for a
    // `Value` (it walks a BTreeMap under the hood for objects), so no
    // extra sorting is required — but we strip trailing whitespace just
    // in case.
    while buf.last() == Some(&b' ') || buf.last() == Some(&b'\n') {
        buf.pop();
    }
    Ok(buf)
}

/// Sign `claims` with `priv_key`, producing a `LicenseFile` carrying an
/// RSASSA-PKCS1-v1_5-SHA256 signature.
pub fn sign_license(
    claims: &LicenseClaims,
    priv_key: &RsaPrivateKey,
) -> Result<LicenseFile, LicenseError> {
    use rsa::signature::{SignatureEncoding, Signer};
    let payload = canonical_claims_bytes(claims)?;
    let signing_key = SigningKey::<Sha256>::new(priv_key.clone());
    let sig: Signature = signing_key.sign(&payload);
    let sig_bytes: Vec<u8> = sig.to_bytes().as_ref().to_vec();
    Ok(LicenseFile {
        claims: claims.clone(),
        signature: sig_bytes,
        algorithm: "RS256".to_string(),
    })
}

/// Verify the signature on `license` using `pub_key`. On success, returns
/// a clone of the inner claims.
pub fn verify_license(
    license: &LicenseFile,
    pub_key: &RsaPublicKey,
) -> Result<LicenseClaims, LicenseError> {
    use rsa::signature::Verifier;
    let payload = canonical_claims_bytes(&license.claims)?;
    let verifying_key = VerifyingKey::<Sha256>::new(pub_key.clone());
    let sig = Signature::try_from(license.signature.as_slice())
        .map_err(|_| LicenseError::InvalidSignature)?;
    verifying_key
        .verify(&payload, &sig)
        .map_err(|_| LicenseError::InvalidSignature)?;
    Ok(license.claims.clone())
}

/// High-level license status check used by the application bootstrap.
///
/// - `license`: `None` if no license file is present on disk → returns
///   `Trial` (the caller then consults `ReportCounter` to see if the trial
///   quota is exhausted).
/// - `pub_key`: the application's bundled public key (used to verify the
///   license signature).
/// - `machine_id`, `site_id`: the current machine's fingerprint.
/// - `trial_quota`: number of reports allowed during a trial (only consulted
///   when `license` is `None` and the caller has already determined the
///   trial quota is exhausted; this function returns `Trial` and lets the
///   caller escalate to `Exhausted` via `ReportCounter`).
pub fn check_status(
    license: Option<&LicenseFile>,
    pub_key: &RsaPublicKey,
    machine_id: &str,
    site_id: &str,
    trial_quota: u32,
) -> LicenseStatus {
    let _ = trial_quota; // caller-side responsibility via ReportCounter

    let Some(license) = license else {
        return LicenseStatus::Trial;
    };

    // Verify the signature first — if it doesn't verify, nothing else matters.
    let claims = match verify_license(license, pub_key) {
        Ok(c) => c,
        Err(_) => return LicenseStatus::Invalid,
    };

    // Machine fingerprint must match.
    if claims.machine_id != machine_id {
        return LicenseStatus::Invalid;
    }
    if let Some(lic_site) = &claims.site_id {
        if lic_site != site_id {
            return LicenseStatus::Invalid;
        }
    }

    // Expiry check.
    let now = current_unix_seconds();
    if let Some(exp) = claims.expires_at {
        if now >= exp {
            return LicenseStatus::Expired;
        }
    }

    // Reports remaining check.
    if let Some(remaining) = claims.reports_remaining {
        if remaining == 0 {
            return LicenseStatus::Exhausted;
        }
    }

    LicenseStatus::Active
}

/// Return the current Unix time in seconds. Uses `std::time::SystemTime`
/// so there are no extra dependencies.
pub fn current_unix_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Compute a stable machine fingerprint from `machine_id` and `site_id`
/// strings. The `machine_id` is typically a SHA-256 of host metadata
/// gathered by the application shell.
pub fn compute_machine_fingerprint(machine_id: &str, site_id: &str) -> MachineFingerprint {
    // Hash the inputs together to produce a stable identifier even if the
    // caller passed a raw hostname rather than a pre-hashed identifier.
    let mut hasher = Sha256::new();
    hasher.update(machine_id.as_bytes());
    hasher.update(b"|");
    hasher.update(site_id.as_bytes());
    let digest = hasher.finalize();
    let fingerprint = digest.iter().map(|b| format!("{:02x}", b)).collect::<String>();
    MachineFingerprint {
        machine_id: if machine_id.len() == 64 && machine_id.chars().all(|c| c.is_ascii_hexdigit()) {
            machine_id.to_string()
        } else {
            fingerprint
        },
        site_id: site_id.to_string(),
    }
}

// --- PEM import / export ------------------------------------------------

/// Export a private key as a PKCS#8 PEM string.
pub fn export_private_key_pem(priv_key: &RsaPrivateKey) -> Result<String, LicenseError> {
    let pem = priv_key.to_pkcs8_pem(LineEnding::LF)?;
    Ok(pem.as_str().to_string())
}

/// Import a private key from a PKCS#8 PEM string.
pub fn import_private_key_pem(pem: &str) -> Result<RsaPrivateKey, LicenseError> {
    Ok(RsaPrivateKey::from_pkcs8_pem(pem)?)
}

/// Export a public key as a SPKI PEM string.
pub fn export_public_key_pem(pub_key: &RsaPublicKey) -> Result<String, LicenseError> {
    let pem = pub_key.to_public_key_pem(LineEnding::LF)?;
    Ok(pem)
}

/// Import a public key from a SPKI PEM string.
pub fn import_public_key_pem(pem: &str) -> Result<RsaPublicKey, LicenseError> {
    Ok(RsaPublicKey::from_public_key_pem(pem)?)
}

/// Save a license file as pretty-printed JSON at `path`.
pub fn save_license_file(path: &Path, license: &LicenseFile) -> Result<(), LicenseError> {
    let json = serde_json::to_string_pretty(license)?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Load a license file from JSON at `path`.
pub fn load_license_file(path: &Path) -> Result<LicenseFile, LicenseError> {
    let bytes = std::fs::read(path)?;
    let license = serde_json::from_slice(&bytes)?;
    Ok(license)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_claims() -> LicenseClaims {
        LicenseClaims {
            license_id: "test-license-001".to_string(),
            customer: "Test Mining Co".to_string(),
            machine_id: "abc123def456".to_string(),
            site_id: Some("SITE-001".to_string()),
            issued_at: 1_700_000_000,
            expires_at: Some(1_800_000_000),
            reports_remaining: Some(10),
        }
    }

    #[test]
    fn test_sign_and_verify_round_trip() {
        let (priv_key, pub_key) = generate_license_keypair().unwrap();
        let claims = sample_claims();
        let license = sign_license(&claims, &priv_key).unwrap();
        assert_eq!(license.algorithm, "RS256");
        assert!(!license.signature.is_empty());
        let verified = verify_license(&license, &pub_key).unwrap();
        assert_eq!(verified.license_id, claims.license_id);
        assert_eq!(verified.reports_remaining, Some(10));
    }

    #[test]
    fn test_tampered_claims_fail_verification() {
        let (priv_key, pub_key) = generate_license_keypair().unwrap();
        let mut license = sign_license(&sample_claims(), &priv_key).unwrap();
        // Tamper with the customer name after signing.
        license.claims.customer = "Evil Corp".to_string();
        let result = verify_license(&license, &pub_key);
        assert!(matches!(result, Err(LicenseError::InvalidSignature)));
    }

    #[test]
    fn test_check_status_trial_when_no_license() {
        let (_, pub_key) = generate_license_keypair().unwrap();
        let status = check_status(None, &pub_key, "abc", "SITE-001", 5);
        assert_eq!(status, LicenseStatus::Trial);
    }

    #[test]
    fn test_check_status_active_for_valid_license() {
        let (priv_key, pub_key) = generate_license_keypair().unwrap();
        let claims = sample_claims();
        let license = sign_license(&claims, &priv_key).unwrap();
        let status = check_status(Some(&license), &pub_key, "abc123def456", "SITE-001", 5);
        assert_eq!(status, LicenseStatus::Active);
    }

    #[test]
    fn test_check_status_invalid_for_wrong_machine() {
        let (priv_key, pub_key) = generate_license_keypair().unwrap();
        let license = sign_license(&sample_claims(), &priv_key).unwrap();
        let status = check_status(Some(&license), &pub_key, "WRONG", "SITE-001", 5);
        assert_eq!(status, LicenseStatus::Invalid);
    }

    #[test]
    fn test_check_status_expired_for_past_expiry() {
        let (priv_key, pub_key) = generate_license_keypair().unwrap();
        let mut claims = sample_claims();
        claims.expires_at = Some(1); // 1970-01-01
        let license = sign_license(&claims, &priv_key).unwrap();
        let status = check_status(Some(&license), &pub_key, "abc123def456", "SITE-001", 5);
        assert_eq!(status, LicenseStatus::Expired);
    }

    #[test]
    fn test_check_status_exhausted_when_zero_reports_remaining() {
        let (priv_key, pub_key) = generate_license_keypair().unwrap();
        let mut claims = sample_claims();
        claims.reports_remaining = Some(0);
        let license = sign_license(&claims, &priv_key).unwrap();
        let status = check_status(Some(&license), &pub_key, "abc123def456", "SITE-001", 5);
        assert_eq!(status, LicenseStatus::Exhausted);
    }

    #[test]
    fn test_pem_round_trip() {
        let (priv_key, pub_key) = generate_license_keypair().unwrap();
        let priv_pem = export_private_key_pem(&priv_key).unwrap();
        let pub_pem = export_public_key_pem(&pub_key).unwrap();
        assert!(priv_pem.contains("BEGIN PRIVATE KEY"));
        assert!(pub_pem.contains("BEGIN PUBLIC KEY"));
        let priv_back = import_private_key_pem(&priv_pem).unwrap();
        let pub_back = import_public_key_pem(&pub_pem).unwrap();
        // Round-trip should preserve the public key.
        let round_trip_pub = RsaPublicKey::from(&priv_back);
        assert_eq!(round_trip_pub, pub_back);
        assert_eq!(pub_back, pub_key);
    }

    #[test]
    fn test_machine_fingerprint_is_stable() {
        let fp1 = compute_machine_fingerprint("host-abc", "SITE-1");
        let fp2 = compute_machine_fingerprint("host-abc", "SITE-1");
        assert_eq!(fp1.machine_id, fp2.machine_id);
        let fp3 = compute_machine_fingerprint("host-xyz", "SITE-1");
        assert_ne!(fp1.machine_id, fp3.machine_id);
    }

    #[test]
    fn test_license_file_save_load_round_trip() {
        let (priv_key, _) = generate_license_keypair().unwrap();
        let license = sign_license(&sample_claims(), &priv_key).unwrap();
        let tmp = tempfile::NamedTempFile::new().unwrap();
        save_license_file(tmp.path(), &license).unwrap();
        let loaded = load_license_file(tmp.path()).unwrap();
        assert_eq!(loaded.claims.license_id, license.claims.license_id);
        assert_eq!(loaded.signature, license.signature);
    }
}
