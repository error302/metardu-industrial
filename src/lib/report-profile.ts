/**
 * Report profile helper — Sprint 20.
 *
 * Loads the user profile from the Rust backend and injects the
 * surveyor's name, company, and registration number into the ReportSpec
 * before it's sent to `generate_report_cmd`.
 *
 * This ensures every PDF report includes the chain-of-custody information
 * (who generated it, from which company, with what registration #).
 */

import { invoke } from "@tauri-apps/api/core";
import { isNative } from "@/lib/tauri-ipc";

interface UserProfile {
  user_id: string;
  name: string;
  email: string;
  company: string;
  registration_number: string | null;
  phone: string | null;
  created_at: number;
  updated_at: number;
  onboarded: boolean;
  license_key: string | null;
  license_tier: string | null;
}

/**
 * Cached profile — loaded once per session to avoid repeated IPC calls.
 */
let cachedProfile: UserProfile | null = null;

/**
 * Load the user profile from the backend. Cached after first call.
 * Returns null in browser mode or if no profile exists.
 */
export async function getReportProfile(): Promise<UserProfile | null> {
  if (cachedProfile !== null) return cachedProfile;
  if (!isNative()) return null;

  try {
    cachedProfile = await invoke<UserProfile>("get_profile_cmd");
    if (!cachedProfile.user_id) {
      // New user — no profile yet
      return null;
    }
    return cachedProfile;
  } catch {
    return null;
  }
}

/**
 * Inject surveyor profile data into a ReportSpec-like object.
 *
 * Usage:
 * ```ts
 * const spec = { ...mySpec, ...await withReportProfile() };
 * await generateReport(spec);
 * ```
 *
 * Returns an object with surveyor_name, surveyor_company, surveyor_registration
 * fields that can be spread into the spec.
 */
export async function withReportProfile(): Promise<{
  surveyor_name: string;
  surveyor_company: string;
  surveyor_registration: string | null;
}> {
  const profile = await getReportProfile();
  if (!profile) {
    return {
      surveyor_name: "",
      surveyor_company: "",
      surveyor_registration: null,
    };
  }
  return {
    surveyor_name: profile.name,
    surveyor_company: profile.company,
    surveyor_registration: profile.registration_number,
  };
}
