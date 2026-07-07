/**
 * License Manager Dialog — Sprint 7.
 *
 * Activation UI for Pro/Enterprise licenses. Users paste a license
 * string (received from sales) or browse to a license file, then
 * activate. The license status is checked on app startup and the
 * badge in the title bar reflects the tier.
 *
 * Also shows feature list so users can see what's unlocked.
 */

import { useState, useEffect } from "react";
import {
  ShieldCheck, Loader2, CheckCircle2, Key, Crown, Lock,
} from "lucide-react";
import { colors } from "@/lib/tokens";
import { DialogShell, DialogButton } from "@/components/dialog-shell";
import {
  getLicenseStatus,
  activateLicense,
  type LicenseStatus,
  type LicenseTier,
} from "@/lib/tauri-ipc";

interface Props {
  open: boolean;
  onClose: () => void;
}

const TIER_LABELS: Record<LicenseTier, string> = {
  core: "Core (Free)",
  pro: "Pro",
  enterprise: "Enterprise",
  trial: "Trial",
};

const TIER_COLORS: Record<LicenseTier, string> = {
  core: "#64748B",
  pro: "#FFA500",
  enterprise: "#6366F1",
  trial: "#F59E0B",
};

const PRO_FEATURES = [
  "EoM Reconciliation",
  "Dredge Pay-Volume Audit",
  "S-44 Compliance Certificate",
  "Stockpile Inventory Audit",
  "Blast Fragmentation Report",
  "Highwall Deformation Monitoring",
  "Survey Deliverable Package",
  "Cross-Section Profiler",
  "Branded PDF Reports",
];

const ENTERPRISE_FEATURES = [
  "Distributed Processing (TCP coordinator)",
  "Plugin SDK (dynamic loading)",
  "Multi-user PostGIS Sync",
  "Custom Branding",
  "Priority Support",
];

export function LicenseManagerDialog({ open, onClose }: Props) {
  const [status, setStatus] = useState<LicenseStatus | null>(null);
  const [licenseText, setLicenseText] = useState("");
  const [activating, setActivating] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);

  useEffect(() => {
    if (open) {
      getLicenseStatus().then(setStatus);
    }
  }, [open]);


  async function handleActivate() {
    setActivating(true);
    setError(null);
    setSuccess(null);
    try {
      const newStatus = await activateLicense(licenseText.trim());
      if (newStatus) {
        setStatus(newStatus);
        if (newStatus.valid) {
          setSuccess(`License activated: ${TIER_LABELS[newStatus.tier]} tier`);
        } else {
          setError(newStatus.error || "License invalid");
        }
      } else {
        setError("Browser mode — license activation requires the native Tauri shell");
      }
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setActivating(false);
    }
  }

  const currentTier = status?.tier ?? "core";
  const tierColor = TIER_COLORS[currentTier];

return (
    <DialogShell
      open={open}
      onClose={onClose}
      title="License Manager"
      icon={<Key className="h-4 w-4" />}
      iconColor={colors.steelLight}
      maxWidth="max-w-2xl"
      subtitle="RSA-PSS signed licenses"
      footerHint="Core/Pro/Enterprise/Trial"
      actions={
        <DialogButton variant="secondary" onClick={onClose}>Close</DialogButton>
      }
    >
          {error && (
            <div className="rounded-md border p-3 text-xs"
              style={{ borderColor: `${colors.fail}40`, background: `${colors.fail}10`, color: colors.fail }}>
              {error}
            </div>
          )}
          {success && (
            <div className="rounded-md border p-3 text-xs"
              style={{ borderColor: `${colors.pass}40`, background: `${colors.pass}10`, color: colors.pass }}>
              {success}
            </div>
          )}

          {/* Current license status */}
          <div className="rounded-md border p-4" style={{ borderColor: `${tierColor}40`, background: `${tierColor}10` }}>
            <div className="flex items-center gap-3">
              {status?.valid ? (
                <ShieldCheck className="h-8 w-8" style={{ color: tierColor }} />
              ) : (
                <Lock className="h-8 w-8" style={{ color: tierColor }} />
              )}
              <div>
                <div className="text-[10px] uppercase tracking-wider" style={{ color: tierColor }}>
                  Current License
                </div>
                <div className="text-lg font-bold text-white">
                  {TIER_LABELS[currentTier]}
                </div>
                {status?.payload && (
                  <div className="text-[10px] text-steel-gray">
                    {status.payload.customer} · ID: {status.payload.license_id.slice(0, 8)}…
                    {status.days_remaining !== null && status.days_remaining > 0 && (
                      <> · {status.days_remaining} days remaining</>
                    )}
                  </div>
                )}
              </div>
            </div>
          </div>

          {/* Activation input */}
          <div>
            <label className="mb-1.5 block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
              Activate License (paste license content)
            </label>
            <textarea
              value={licenseText}
              onChange={(e) => setLicenseText(e.target.value)}
              rows={6}
              placeholder={'Paste the license content you received from MetaRDU Sales here...\n\nIt should look like:\n{\n  "customer": "...",\n  "tier": "pro",\n  ...\n}\nSIGNATURE: hex_hmac_string'}
              className="input-enterprise w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 font-mono text-xs text-white focus:outline-none"
            />
            <button
              onClick={handleActivate}
              disabled={!licenseText.trim() || activating}
              className="mt-2 flex items-center gap-2 rounded-md px-4 py-2 text-sm font-bold transition-colors disabled:opacity-40"
              style={{ background: colors.industrialOrange, color: colors.navyBase }}
            >
              {activating ? <Loader2 className="h-4 w-4 animate-spin" /> : <Key className="h-4 w-4" />}
              {activating ? "Activating…" : "Activate License"}
            </button>
          </div>

          {/* Feature comparison */}
          <div className="grid grid-cols-2 gap-3">
            <FeatureColumn
              title="Pro Tier"
              color={TIER_COLORS.pro}
              icon={<Crown className="h-4 w-4" />}
              features={PRO_FEATURES}
              unlocked={status?.tier === "pro" || status?.tier === "enterprise" || status?.tier === "trial"}
              price="$3,000-5,000/seat/year"
            />
            <FeatureColumn
              title="Enterprise Tier"
              color={TIER_COLORS.enterprise}
              icon={<ShieldCheck className="h-4 w-4" />}
              features={[...PRO_FEATURES, ...ENTERPRISE_FEATURES]}
              unlocked={status?.tier === "enterprise"}
              price="$10,000-25,000/site/year"
            />
          </div>

          {/* Contact sales */}
          <div className="input-enterprise rounded-md border border-navy-border bg-navy-base p-3 text-xs text-steel-light">
            <div className="font-semibold text-white">Need a license?</div>
            <div className="mt-1">
              Contact MetaRDU Industrial Sales: sales@metardu.example
              <br />
              Or visit: https://metardu.example/pricing
            </div>
          </div>
    </DialogShell>
  );
}

function FeatureColumn({
  title, color, icon, features, unlocked, price,
}: {
  title: string; color: string; icon: React.ReactNode;
  features: string[]; unlocked: boolean; price: string;
}) {
  return (
    <div className="rounded-md border p-3"
      style={{
        borderColor: unlocked ? `${color}80` : colors.navyBorder,
        background: unlocked ? `${color}10` : colors.navyBase,
      }}
    >
      <div className="mb-2 flex items-center gap-2">
        <span style={{ color }}>{icon}</span>
        <span className="text-sm font-bold text-white">{title}</span>
        {unlocked && <CheckCircle2 className="ml-auto h-3.5 w-3.5" style={{ color: colors.pass }} />}
      </div>
      <div className="text-[10px] text-steel-gray mb-2">{price}</div>
      <div className="space-y-1">
        {features.map((f, i) => (
          <div key={i} className="flex items-start gap-1.5 text-[10px]">
            <span style={{ color: unlocked ? colors.pass : colors.steelGray }}>
              {unlocked ? <CheckCircle2 className="h-3 w-3 mt-0.5 flex-shrink-0" /> : <Lock className="h-3 w-3 mt-0.5 flex-shrink-0" />}
            </span>
            <span className={unlocked ? "text-steel-light" : "text-steel-gray"}>{f}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
