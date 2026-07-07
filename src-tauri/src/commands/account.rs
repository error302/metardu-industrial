// Account IPC commands — Sprint 20.
//
// Exposes the user profile management to the frontend:
//   - get_profile: load the current user profile (empty if new user)
//   - create_account: register a new user
//   - update_profile: edit name/email/company/etc.
//   - link_license: associate a license key with the profile
//   - delete_account: remove all profile data (account deletion)

use crate::account::{create_account, delete_profile, link_license, load_profile, update_profile, UserProfile};

/// Get the current user profile. Returns an empty profile if no account
/// exists yet (new user). The frontend uses this to decide whether to
/// show the onboarding/account-creation flow.
#[tauri::command]
pub fn get_profile_cmd() -> UserProfile {
    load_profile()
}

/// Create a new user account. Called from the onboarding screen on
/// first launch, or from Settings → Account → "Create Account".
#[tauri::command]
pub fn create_account_cmd(
    name: String,
    email: String,
    company: String,
    registration_number: Option<String>,
    phone: Option<String>,
) -> Result<UserProfile, String> {
    create_account(name, email, company, registration_number, phone)
}

/// Update the user profile. All fields are optional — only provided
/// fields are updated.
#[tauri::command]
pub fn update_profile_cmd(
    name: Option<String>,
    email: Option<String>,
    company: Option<String>,
    registration_number: Option<Option<String>>,
    phone: Option<Option<String>>,
) -> Result<UserProfile, String> {
    update_profile(name, email, company, registration_number, phone)
}

/// Associate a license key + tier with the user profile. Called after
/// successful license activation via the License Manager dialog.
#[tauri::command]
pub fn link_license_cmd(key: String, tier: String) -> Result<UserProfile, String> {
    link_license(key, tier)
}

/// Delete the user profile entirely. Used for account deletion / reset.
/// Does NOT delete the license file (that's separate).
#[tauri::command]
pub fn delete_account_cmd() -> Result<(), String> {
    delete_profile()
}

/// Check if the user has completed onboarding (account created + onboarded flag set).
/// The frontend uses this on app launch to decide whether to show the
/// onboarding screen or go straight to the workspace.
#[tauri::command]
pub fn is_onboarded_cmd() -> bool {
    let profile = load_profile();
    profile.onboarded && profile.is_complete()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_profile_new_user() {
        let dir = std::env::temp_dir();
        std::env::set_var("XDG_DATA_HOME", &dir);
        let _ = delete_profile();
        let profile = get_profile_cmd();
        assert!(profile.is_new());
    }

    #[test]
    fn test_create_and_get_profile() {
        let dir = std::env::temp_dir();
        std::env::set_var("XDG_DATA_HOME", &dir);
        let _ = delete_profile();

        create_account_cmd(
            "Test Surveyor".to_string(),
            "surveyor@example.com".to_string(),
            "Mining Co".to_string(),
            None,
            None,
        )
        .unwrap();

        let profile = get_profile_cmd();
        assert_eq!(profile.name, "Test Surveyor");
        assert!(profile.onboarded);

        let _ = delete_profile();
    }

    #[test]
    fn test_is_onboarded() {
        let dir = std::env::temp_dir();
        std::env::set_var("XDG_DATA_HOME", &dir);
        let _ = delete_profile();

        assert!(!is_onboarded_cmd());

        create_account_cmd(
            "Test".to_string(),
            "test@test.com".to_string(),
            "Co".to_string(),
            None,
            None,
        )
        .unwrap();

        assert!(is_onboarded_cmd());
        let _ = delete_profile();
    }
}
