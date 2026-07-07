/**
 * Account Dialog — Sprint 20.
 *
 * User registration + profile management. On first launch, the onboarding
 * screen shows this dialog for account creation. After that, it's
 * accessible via Settings → Account.
 *
 * The profile stores: name, email, company, registration #, phone.
 * This is NOT server-side authentication — the license key IS the auth.
 * The profile enriches PDF reports with the surveyor's identity.
 *
 * Flow:
 *   1. New user → "Create Account" form (name*, email*, company*, reg#, phone)
 *   2. Existing user → "Edit Profile" form (same fields, pre-filled)
 *   3. License section → shows current tier + key + "Activate License" button
 */

import { useState, useEffect } from "react";
import { User, Loader2, CheckCircle2, AlertCircle, Key, Shield } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { colors } from "@/lib/tokens";
import { isNative } from "@/lib/tauri-ipc";
import { DialogShell, DialogButton } from "@/components/dialog-shell";

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

interface Props {
  open: boolean;
  onClose: () => void;
  /** If true, this is the first-launch onboarding flow (can't be dismissed). */
  isOnboarding?: boolean;
  /** Called after account creation/update so the parent can refresh state. */
  onProfileChanged?: (profile: UserProfile) => void;
}

export function AccountDialog({ open, onClose, isOnboarding = false, onProfileChanged }: Props) {
  const [profile, setProfile] = useState<UserProfile | null>(null);
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [company, setCompany] = useState("");
  const [registrationNumber, setRegistrationNumber] = useState("");
  const [phone, setPhone] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);

  useEffect(() => {
    if (open) {
      loadProfile();
    }
  }, [open]); // eslint-disable-line react-hooks/exhaustive-deps

  async function loadProfile() {
    if (!isNative()) return;
    try {
      const p = await invoke<UserProfile>("get_profile_cmd");
      setProfile(p);
      setName(p.name);
      setEmail(p.email);
      setCompany(p.company);
      setRegistrationNumber(p.registration_number ?? "");
      setPhone(p.phone ?? "");
    } catch (err) {
      // New user — no profile yet
    }
  }

  async function handleSave() {
    setLoading(true);
    setError(null);
    setSuccess(null);
    try {
      if (!isNative()) {
        setError("Browser mode — account management requires the native Tauri shell");
        return;
      }

      const isNew = !profile || !profile.user_id;
      if (isNew) {
        // Create new account
        const result = await invoke<UserProfile>("create_account_cmd", {
          name,
          email,
          company,
          registrationNumber: registrationNumber.trim() || null,
          phone: phone.trim() || null,
        });
        setProfile(result);
        setSuccess("Account created successfully!");
        onProfileChanged?.(result);
      } else {
        // Update existing profile
        const result = await invoke<UserProfile>("update_profile_cmd", {
          name,
          email,
          company,
          registrationNumber: registrationNumber.trim() || null,
          phone: phone.trim() || null,
        });
        setProfile(result);
        setSuccess("Profile updated!");
        onProfileChanged?.(result);
      }
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }

  const isNew = !profile || !profile.user_id;
  const licenseTier = profile?.license_tier ?? "Core (Free)";

  return (
    <DialogShell
      open={open}
      onClose={isOnboarding ? () => {} : onClose}
      title={isNew ? "Create Account" : "Your Account"}
      icon={<User className="h-4 w-4" />}
      iconColor={colors.accent}
      maxWidth="max-w-lg"
      subtitle={isNew ? "Register to start using MetaRDU Industrial" : `${profile?.name} · ${licenseTier}`}
      footerHint="Your profile is stored locally and used in PDF reports. No server required."
      actions={
        <>
          {!isOnboarding && <DialogButton variant="secondary" onClick={onClose}>Close</DialogButton>}
          <DialogButton
            variant="primary"
            onClick={handleSave}
            disabled={loading || !name.trim() || !email.trim() || !company.trim()}
          >
            {loading ? <Loader2 className="h-3 w-3 animate-spin" /> : <CheckCircle2 className="h-3 w-3" />}
            {loading ? "Saving…" : isNew ? "Create Account" : "Save Changes"}
          </DialogButton>
        </>
      }
    >
      <div className="space-y-4">
        {/* Account info (existing users only) */}
        {!isNew && profile && (
          <div className="grid grid-cols-2 gap-2 rounded-md border p-3" style={{ borderColor: `${colors.accent}40`, background: `${colors.accent}08` }}>
            <div>
              <div className="text-[9px] uppercase tracking-wider text-steel-gray">User ID</div>
              <div className="font-mono text-[10px] text-steel-light">{profile.user_id}</div>
            </div>
            <div>
              <div className="text-[9px] uppercase tracking-wider text-steel-gray">Member since</div>
              <div className="font-mono text-[10px] text-steel-light">
                {new Date(profile.created_at).toLocaleDateString()}
              </div>
            </div>
          </div>
        )}

        {/* Form fields */}
        <div className="space-y-3">
          <div>
            <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
              Full Name <span style={{ color: colors.fail }}>*</span>
            </label>
            <input
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="Sarah Mitchell"
              className="input-enterprise w-full rounded-md border border-navy-border bg-navy-base px-3 py-1.5 text-sm text-white focus:border-accent focus:outline-none"
            />
          </div>

          <div>
            <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
              Email <span style={{ color: colors.fail }}>*</span>
            </label>
            <input
              type="email"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              placeholder="sarah@mine.com"
              className="input-enterprise w-full rounded-md border border-navy-border bg-navy-base px-3 py-1.5 text-sm text-white focus:border-accent focus:outline-none"
            />
            <p className="mt-0.5 text-[9px] text-steel-gray">Used for license recovery and report contact</p>
          </div>

          <div>
            <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
              Company / Organization <span style={{ color: colors.fail }}>*</span>
            </label>
            <input
              type="text"
              value={company}
              onChange={(e) => setCompany(e.target.value)}
              placeholder="BHP Gold Mine — WA"
              className="input-enterprise w-full rounded-md border border-navy-border bg-navy-base px-3 py-1.5 text-sm text-white focus:border-accent focus:outline-none"
            />
          </div>

          <div className="grid grid-cols-2 gap-3">
            <div>
              <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
                Registration No.
              </label>
              <input
                type="text"
                value={registrationNumber}
                onChange={(e) => setRegistrationNumber(e.target.value)}
                placeholder="SV12345"
                className="input-enterprise w-full rounded-md border border-navy-border bg-navy-base px-3 py-1.5 font-mono text-xs text-white focus:border-accent focus:outline-none"
              />
            </div>
            <div>
              <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
                Phone
              </label>
              <input
                type="tel"
                value={phone}
                onChange={(e) => setPhone(e.target.value)}
                placeholder="+61 400 000 000"
                className="input-enterprise w-full rounded-md border border-navy-border bg-navy-base px-3 py-1.5 font-mono text-xs text-white focus:border-accent focus:outline-none"
              />
            </div>
          </div>
        </div>

        {/* License section */}
        {!isNew && (
          <div className="input-enterprise rounded-md border border-navy-border bg-navy-base p-3">
            <div className="mb-2 flex items-center gap-1.5 text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
              <Shield className="h-3 w-3" /> License
            </div>
            <div className="flex items-center justify-between">
              <div>
                <div className="text-sm font-semibold text-white">{licenseTier}</div>
                {profile?.license_key && (
                  <div className="font-mono text-[10px] text-steel-gray">
                    Key: {profile.license_key.slice(0, 8)}••••••••
                  </div>
                )}
              </div>
              <Key className="h-4 w-4" style={{ color: profile?.license_key ? colors.pass : colors.steelGray }} />
            </div>
            {!profile?.license_key && (
              <p className="mt-1.5 text-[10px] text-steel-gray">
                No license activated. Using Core (free) tier. Activate a license key via
                the License Manager to unlock Pro features.
              </p>
            )}
          </div>
        )}

        {/* Error / success messages */}
        {error && (
          <div className="flex items-center gap-2 rounded-md border p-3 text-xs" style={{ borderColor: `${colors.fail}40`, background: `${colors.fail}10`, color: colors.fail }}>
            <AlertCircle className="h-4 w-4 flex-shrink-0" />
            {error}
          </div>
        )}
        {success && (
          <div className="flex items-center gap-2 rounded-md border p-3 text-xs" style={{ borderColor: `${colors.pass}40`, background: `${colors.pass}10`, color: colors.pass }}>
            <CheckCircle2 className="h-4 w-4 flex-shrink-0" />
            {success}
          </div>
        )}

        {/* Privacy note */}
        <div className="rounded-md bg-navy-base p-2 text-[10px] leading-relaxed text-steel-gray">
          <strong className="text-steel-light">Privacy:</strong> Your profile is stored locally on this
          machine only. MetaRDU does not send your personal data to any server. The license key
          is verified locally using RSA-PSS signatures — no internet connection required.
        </div>
      </div>
    </DialogShell>
  );
}
