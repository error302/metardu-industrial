// User account / profile — Sprint 20.
//
// Stores the user's identity locally for:
//   - PDF report generation (surveyor name, company, contact)
//   - License activation (ties a license key to a named user)
//   - Telemetry (anonymous usage stats attributed to a user ID)
//   - Report chain-of-custody (who generated which report)
//
// This is NOT server-side authentication. MetaRDU is a desktop app —
// the license key IS the authentication. The user profile is local
// metadata that enriches reports and ties them to a named individual.
//
// Flow:
//   1. First launch → onboarding → "Create Account" (name, email, company)
//   2. Account saved to app_data_dir/profile.json
//   3. User activates a license key → license tied to machine + profile
//   4. All PDF reports include the user's name + company from the profile
//   5. User can edit their profile via Settings → Account

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    /// Unique user ID (generated on account creation)
    pub user_id: String,
    /// Full name (e.g., "Sarah Mitchell")
    pub name: String,
    /// Email address (used for license recovery + report contact)
    pub email: String,
    /// Company / organization (e.g., "BHP Gold Mine — WA")
    pub company: String,
    /// Professional registration number (e.g., surveyor license #)
    pub registration_number: Option<String>,
    /// Phone (optional, for report contact)
    pub phone: Option<String>,
    /// Account creation timestamp (Unix ms)
    pub created_at: u64,
    /// Last profile update timestamp
    pub updated_at: u64,
    /// Whether the user has completed onboarding
    pub onboarded: bool,
    /// License key associated with this profile (if any)
    pub license_key: Option<String>,
    /// License tier associated with this profile
    pub license_tier: Option<String>,
}

impl Default for UserProfile {
    fn default() -> Self {
        Self {
            user_id: String::new(),
            name: String::new(),
            email: String::new(),
            company: String::new(),
            registration_number: None,
            phone: None,
            created_at: 0,
            updated_at: 0,
            onboarded: false,
            license_key: None,
            license_tier: None,
        }
    }
}

impl UserProfile {
    /// Check if the profile is complete (has name + email + company)
    pub fn is_complete(&self) -> bool {
        !self.name.is_empty() && !self.email.is_empty() && !self.company.is_empty()
    }

    /// Check if this is a new user (no profile created yet)
    pub fn is_new(&self) -> bool {
        self.user_id.is_empty()
    }
}

/// Get the profile file path (app_data_dir/profile.json).
fn profile_path() -> PathBuf {
    let base = if cfg!(target_os = "windows") {
        std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string())
    } else if cfg!(target_os = "macos") {
        std::env::var("HOME").unwrap_or_else(|_| ".".to_string()) + "/Library/Application Support"
    } else {
        std::env::var("XDG_DATA_HOME").unwrap_or_else(|_| {
            std::env::var("HOME").unwrap_or_else(|_| ".".to_string()) + "/.local/share"
        })
    };
    PathBuf::from(base)
        .join("metardu-industrial")
        .join("profile.json")
}

/// Load the user profile from disk. Returns a default (empty) profile
/// if no profile exists yet (new user).
pub fn load_profile() -> UserProfile {
    let path = profile_path();
    if !path.exists() {
        return UserProfile::default();
    }
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => UserProfile::default(),
    }
}

/// Save the user profile to disk.
pub fn save_profile(profile: &UserProfile) -> Result<(), String> {
    let path = profile_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("creating profile dir: {e}"))?;
    }
    let json = serde_json::to_string_pretty(profile)
        .map_err(|e| format!("serializing profile: {e}"))?;
    std::fs::write(&path, json).map_err(|e| format!("writing profile: {e}"))?;
    Ok(())
}

/// Create a new user account. Generates a unique user ID and saves
/// the profile.
pub fn create_account(
    name: String,
    email: String,
    company: String,
    registration_number: Option<String>,
    phone: Option<String>,
) -> Result<UserProfile, String> {
    if name.trim().is_empty() {
        return Err("Name is required".to_string());
    }
    if email.trim().is_empty() || !email.contains('@') {
        return Err("A valid email address is required".to_string());
    }
    if company.trim().is_empty() {
        return Err("Company / organization is required".to_string());
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    let profile = UserProfile {
        user_id: generate_user_id(&email),
        name: name.trim().to_string(),
        email: email.trim().to_string(),
        company: company.trim().to_string(),
        registration_number: registration_number.filter(|s| !s.is_empty()),
        phone: phone.filter(|s| !s.is_empty()),
        created_at: now,
        updated_at: now,
        onboarded: true,
        license_key: None,
        license_tier: Some("Core".to_string()),
    };

    save_profile(&profile)?;
    Ok(profile)
}

/// Update an existing user profile.
pub fn update_profile(
    name: Option<String>,
    email: Option<String>,
    company: Option<String>,
    registration_number: Option<Option<String>>,
    phone: Option<Option<String>>,
) -> Result<UserProfile, String> {
    let mut profile = load_profile();
    if profile.is_new() {
        return Err("No account exists. Create an account first.".to_string());
    }

    if let Some(n) = name {
        if n.trim().is_empty() {
            return Err("Name cannot be empty".to_string());
        }
        profile.name = n.trim().to_string();
    }
    if let Some(e) = email {
        if e.trim().is_empty() || !e.contains('@') {
            return Err("A valid email address is required".to_string());
        }
        profile.email = e.trim().to_string();
    }
    if let Some(c) = company {
        if c.trim().is_empty() {
            return Err("Company cannot be empty".to_string());
        }
        profile.company = c.trim().to_string();
    }
    if let Some(rn) = registration_number {
        profile.registration_number = rn.filter(|s| !s.is_empty());
    }
    if let Some(p) = phone {
        profile.phone = p.filter(|s| !s.is_empty());
    }

    profile.updated_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    save_profile(&profile)?;
    Ok(profile)
}

/// Associate a license key + tier with the user profile.
/// Called after successful license activation.
pub fn link_license(key: String, tier: String) -> Result<UserProfile, String> {
    let mut profile = load_profile();
    if profile.is_new() {
        return Err("No account exists. Create an account first.".to_string());
    }
    profile.license_key = Some(key);
    profile.license_tier = Some(tier);
    profile.updated_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    save_profile(&profile)?;
    Ok(profile)
}

/// Delete the user profile (account deletion / reset).
pub fn delete_profile() -> Result<(), String> {
    let path = profile_path();
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| format!("deleting profile: {e}"))?;
    }
    Ok(())
}

/// Generate a unique user ID from email + timestamp.
/// Format: "usr-<8-char-hash>-<4-char-random>"
fn generate_user_id(email: &str) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);

    // Simple hash from email + timestamp
    let input = format!("{}{}", email, timestamp);
    let mut hash: u64 = 5381;
    for byte in input.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(byte as u64);
    }

    // Random component
    let random = (hash ^ (hash >> 32)) & 0xFFFF;

    format!("usr-{:08x}-{:04x}", (hash & 0xFFFFFFFF) as u32, random as u16)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_profile_is_new() {
        let profile = UserProfile::default();
        assert!(profile.is_new());
        assert!(!profile.is_complete());
        assert!(!profile.onboarded);
    }

    #[test]
    fn test_profile_is_complete() {
        let profile = UserProfile {
            user_id: "usr-test".to_string(),
            name: "Test User".to_string(),
            email: "test@example.com".to_string(),
            company: "Test Co".to_string(),
            ..Default::default()
        };
        assert!(profile.is_complete());
        assert!(!profile.is_new());
    }

    #[test]
    fn test_create_account_validates_name() {
        let result = create_account(
            "".to_string(),
            "test@example.com".to_string(),
            "Test Co".to_string(),
            None,
            None,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Name"));
    }

    #[test]
    fn test_create_account_validates_email() {
        let result = create_account(
            "Test User".to_string(),
            "not-an-email".to_string(),
            "Test Co".to_string(),
            None,
            None,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("email"));
    }

    #[test]
    fn test_create_account_validates_company() {
        let result = create_account(
            "Test User".to_string(),
            "test@example.com".to_string(),
            "".to_string(),
            None,
            None,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Company"));
    }

    #[test]
    fn test_generate_user_id_format() {
        let id = generate_user_id("test@example.com");
        assert!(id.starts_with("usr-"));
        assert_eq!(id.len(), 18); // "usr-" + 8 hex + "-" + 4 hex = 18
    }

    #[test]
    fn test_generate_user_id_unique() {
        let id1 = generate_user_id("test@example.com");
        std::thread::sleep(std::time::Duration::from_millis(10));
        let id2 = generate_user_id("test@example.com");
        // Should be different due to timestamp difference
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_save_and_load_profile() {
        // Use a temp dir for testing
        let dir = std::env::temp_dir();
        std::env::set_var("XDG_DATA_HOME", &dir);

        let profile = create_account(
            "Sarah Mitchell".to_string(),
            "sarah@bhp.com".to_string(),
            "BHP Gold Mine — WA".to_string(),
            Some("SV12345".to_string()),
            Some("+61 400 000 000".to_string()),
        )
        .unwrap();

        assert!(profile.is_complete());
        assert!(profile.onboarded);
        assert_eq!(profile.name, "Sarah Mitchell");
        assert_eq!(profile.email, "sarah@bhp.com");
        assert_eq!(profile.company, "BHP Gold Mine — WA");
        assert_eq!(profile.registration_number, Some("SV12345".to_string()));
        assert_eq!(profile.license_tier, Some("Core".to_string()));

        // Load it back
        let loaded = load_profile();
        assert_eq!(loaded.user_id, profile.user_id);
        assert_eq!(loaded.name, "Sarah Mitchell");
        assert!(loaded.onboarded);

        // Clean up
        let _ = delete_profile();
    }

    #[test]
    fn test_update_profile() {
        let dir = std::env::temp_dir();
        std::env::set_var("XDG_DATA_HOME", &dir);

        create_account(
            "Test User".to_string(),
            "test@example.com".to_string(),
            "Test Co".to_string(),
            None,
            None,
        )
        .unwrap();

        let updated = update_profile(
            Some("Updated Name".to_string()),
            None,
            Some("New Co".to_string()),
            Some(Some("REG-001".to_string())),
            None,
        )
        .unwrap();

        assert_eq!(updated.name, "Updated Name");
        assert_eq!(updated.company, "New Co");
        assert_eq!(updated.registration_number, Some("REG-001".to_string()));
        // Email should be unchanged
        assert_eq!(updated.email, "test@example.com");

        let _ = delete_profile();
    }

    #[test]
    fn test_link_license() {
        let dir = std::env::temp_dir();
        std::env::set_var("XDG_DATA_HOME", &dir);

        create_account(
            "Test User".to_string(),
            "test@example.com".to_string(),
            "Test Co".to_string(),
            None,
            None,
        )
        .unwrap();

        let profile = link_license("PRO-KEY-12345".to_string(), "Pro".to_string()).unwrap();
        assert_eq!(profile.license_key, Some("PRO-KEY-12345".to_string()));
        assert_eq!(profile.license_tier, Some("Pro".to_string()));

        let _ = delete_profile();
    }

    #[test]
    fn test_delete_profile() {
        let dir = std::env::temp_dir();
        std::env::set_var("XDG_DATA_HOME", &dir);

        create_account(
            "Test".to_string(),
            "test@test.com".to_string(),
            "Co".to_string(),
            None,
            None,
        )
        .unwrap();

        delete_profile().unwrap();

        let loaded = load_profile();
        assert!(loaded.is_new());
    }
}
