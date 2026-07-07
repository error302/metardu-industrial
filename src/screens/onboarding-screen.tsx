/**
 * First-Run Onboarding Screen
 * Three steps: domain selection → CRS selection → account creation.
 * Sprint 20: added account creation step (required, can't skip).
 * Skippable for domain/CRS — power users can configure later in Settings.
 * Account creation is required (can't proceed without name + email + company).
 */

import { useState } from "react";
import { Mountain, Ship, ArrowRight, Search, ArrowLeft, CheckCircle2 } from "lucide-react";
import {
  colors,
  domainAccent,
  APP_NAME,
  type DomainMode,
} from "@/lib/tokens";
import { BrandLogoMark } from "@/components/brand-logo";
import { useAppStore } from "@/stores/app-store";
import { useViewport } from "@/lib/use-viewport";
import { invoke } from "@tauri-apps/api/core";
import { isNative } from "@/lib/tauri-ipc";
import { CRS_QUICKPICKS } from "@/lib/crs-quickpicks";

type OnboardingStep = "domain" | "account";

export function OnboardingScreen() {
  const completeOnboarding = useAppStore((s) => s.completeOnboarding);
  const { isNarrow, isVeryNarrow } = useViewport();
  const [domain, setDomain] = useState<DomainMode | null>(null);
  const [epsg, setEpsg] = useState("EPSG:4326");
  const [search, setSearch] = useState("");
  const [step, setStep] = useState<OnboardingStep>("domain");

  // Account fields
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [company, setCompany] = useState("");
  const [regNumber, setRegNumber] = useState("");
  const [phone, setPhone] = useState("");
  const [creating, setCreating] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const filteredCrs = CRS_QUICKPICKS.filter(
    (c) =>
      c.code.toLowerCase().includes(search.toLowerCase()) ||
      c.label.toLowerCase().includes(search.toLowerCase()) ||
      c.datum.toLowerCase().includes(search.toLowerCase()) ||
      c.region.toLowerCase().includes(search.toLowerCase()),
  );

  return (
    <div className="flex h-full w-full flex-col bg-navy-base">
      {/* Header */}
      <header className="flex h-12 items-center justify-between border-b border-navy-border px-4 sm:px-6">
        <div className="flex items-center gap-3 min-w-0">
          <BrandLogoMark size={24} />
          <span className="text-sm font-medium tracking-wide text-white truncate">
            {APP_NAME}
          </span>
        </div>
        <button
          onClick={() =>
            completeOnboarding({ defaultDomain: "both", defaultEpsg: "EPSG:4326" })
          }
          className="text-xs text-steel-gray hover:text-white whitespace-nowrap"
        >
          <span className="hidden sm:inline">Skip onboarding →</span>
          <span className="sm:hidden">Skip →</span>
        </button>
      </header>

      {/* Body */}
      <div className="flex-1 overflow-y-auto">
        {step === "domain" && (
        <div className="mx-auto max-w-3xl px-4 sm:px-8 py-6 sm:py-10">
          <h1 className="text-xl sm:text-2xl font-bold text-white">
            Welcome to {APP_NAME}
          </h1>
          <p className="mt-2 text-sm text-steel-light">
            Configure your default workspace. These choices shape your panels,
            keyboard shortcuts, and color mode. Switchable any time in Settings.
          </p>

          {/* Domain selection */}
          <section className="mt-8 sm:mt-10">
            <h2 className="mb-1 text-sm font-semibold uppercase tracking-wider text-steel-light">
              1 · Which surveys will you be running?
            </h2>
            <p className="mb-5 text-xs text-steel-gray">
              Select both if you serve mining and marine clients.
            </p>

            <div
              className={`grid gap-3 sm:gap-4 ${
                isVeryNarrow ? "grid-cols-1" : isNarrow ? "grid-cols-1" : "grid-cols-3"
              }`}
            >
              <DomainCard
                selected={domain === "mining"}
                onClick={() => setDomain("mining")}
                accent={domainAccent.mining.primary}
                accentSecondary={domainAccent.mining.secondary}
                icon={<Mountain className="h-7 w-7" />}
                title="Mining"
                description="Open-pit and underground surveys: drone photogrammetry, TLS, volumes, blast design, 4D monitoring."
              />
              <DomainCard
                selected={domain === "marine"}
                onClick={() => setDomain("marine")}
                accent={domainAccent.marine.primary}
                accentSecondary={domainAccent.marine.secondary}
                icon={<Ship className="h-7 w-7" />}
                title="Marine"
                description="Hydrographic surveys: multibeam, side scan, CUBE surfaces, S-44 compliance, S-57 export."
              />
              <DomainCard
                selected={domain === "both"}
                onClick={() => setDomain("both")}
                accent={domainAccent.both.primary}
                accentSecondary={domainAccent.both.secondary}
                icon={
                  <div className="relative">
                    <Mountain className="h-7 w-7" />
                    <Ship className="absolute -bottom-1 -right-1 h-4 w-4" />
                  </div>
                }
                title="Both"
                description="Dual-domain contractor mode. Full workflow set for both mining and marine, with split-view comparison."
              />
            </div>
          </section>

          {/* CRS selection */}
          <section className="mt-8 sm:mt-10">
            <h2 className="mb-1 text-sm font-semibold uppercase tracking-wider text-steel-light">
              2 · Default coordinate system
            </h2>
            <p className="mb-5 text-xs text-steel-gray">
              Used for new projects and the initial map view. Mine grids can be
              registered later via Settings → Coordinate Systems.
            </p>

            <div className="rounded-lg border border-navy-border bg-navy-panel">
              <div className="flex items-center gap-2 border-b border-navy-border px-3 py-2">
                <Search className="h-4 w-4 text-steel-gray" />
                <input
                  type="text"
                  value={search}
                  onChange={(e) => setSearch(e.target.value)}
                  placeholder="Search EPSG code or description…"
                  className="flex-1 bg-transparent text-sm text-white placeholder:text-steel-gray focus:outline-none"
                />
              </div>
              <div className="max-h-64 overflow-y-auto p-1">
                {filteredCrs.length === 0 && (
                  <div className="px-3 py-6 text-center text-xs text-steel-gray">
                    No matches. Custom CRS can be added in Settings.
                  </div>
                )}
                {filteredCrs.map((crs) => (
                  <button
                    key={crs.code}
                    onClick={() => setEpsg(crs.code)}
                    className={`flex w-full items-center justify-between rounded-md px-3 py-2 text-left transition-colors ${
                      epsg === crs.code
                        ? "bg-navy-elevated"
                        : "hover:bg-navy-elevated/50"
                    }`}
                  >
                    <div className="flex items-center gap-3">
                      <span
                        className="font-mono text-xs font-semibold"
                        style={{
                          color:
                            epsg === crs.code
                              ? colors.industrialOrange
                              : colors.steelLight,
                        }}
                      >
                        {crs.code}
                      </span>
                      <span className="text-sm text-white">{crs.label}</span>
                    </div>
                    {epsg === crs.code && (
                      <div
                        className="h-2 w-2 rounded-full"
                        style={{ background: colors.industrialOrange }}
                      />
                    )}
                  </button>
                ))}
              </div>
            </div>
          </section>

          {/* Get started */}
          <div className="mt-8 sm:mt-10 flex flex-col sm:flex-row items-start sm:items-center justify-between gap-3">
            <div className="text-xs text-steel-gray">
              {domain ? (
                <>
                  Domain:{" "}
                  <span style={{ color: colors.industrialOrange }}>
                    {domainAccent[domain].label}
                  </span>
                  {" · "}
                  CRS:{" "}
                  <span className="font-mono" style={{ color: colors.steelLight }}>
                    {epsg}
                  </span>
                </>
              ) : (
                "Select a domain to continue"
              )}
            </div>
            <button
              disabled={!domain}
              onClick={() => setStep("account")}
              className="flex items-center gap-2 rounded-md px-4 py-2 text-sm font-medium transition-colors disabled:cursor-not-allowed disabled:opacity-40"
              style={{
                background: domain ? colors.industrialOrange : colors.steelGray,
                color: colors.navyBase,
              }}
            >
              Continue
              <ArrowRight className="h-4 w-4" />
            </button>
          </div>
        </div>
        )}

        {/* Account creation step (Sprint 20) */}
        {step === "account" && (
          <div className="mx-auto max-w-lg px-4 sm:px-8 py-6 sm:py-10">
            <button
              onClick={() => setStep("domain")}
              className="mb-4 flex items-center gap-1 text-xs text-steel-gray hover:text-white"
            >
              <ArrowLeft className="h-3 w-3" /> Back
            </button>
            <h1 className="text-xl sm:text-2xl font-bold text-white">
              Create Your Account
            </h1>
            <p className="mt-2 text-sm text-steel-light">
              Your profile is stored locally and used in PDF reports for chain-of-custody.
              No server required — MetaRDU is a desktop app.
            </p>

            <div className="mt-6 space-y-4">
              <div>
                <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
                  Full Name <span style={{ color: colors.fail }}>*</span>
                </label>
                <input
                  type="text"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  placeholder="Sarah Mitchell"
                  className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 text-sm text-white focus:border-accent focus:outline-none"
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
                  className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 text-sm text-white focus:border-accent focus:outline-none"
                />
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
                  className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 text-sm text-white focus:border-accent focus:outline-none"
                />
              </div>
              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
                    Registration No.
                  </label>
                  <input
                    type="text"
                    value={regNumber}
                    onChange={(e) => setRegNumber(e.target.value)}
                    placeholder="SV12345"
                    className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 font-mono text-xs text-white focus:border-accent focus:outline-none"
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
                    className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 font-mono text-xs text-white focus:border-accent focus:outline-none"
                  />
                </div>
              </div>

              {error && (
                <div className="rounded-md border p-3 text-xs" style={{ borderColor: `${colors.fail}40`, background: `${colors.fail}10`, color: colors.fail }}>
                  {error}
                </div>
              )}

              <div className="rounded-md bg-navy-base p-2 text-[10px] leading-relaxed text-steel-gray">
                <strong className="text-steel-light">Privacy:</strong> Your profile is stored locally on this
                machine only. MetaRDU does not send your personal data to any server.
              </div>

              <button
                disabled={creating || !name.trim() || !email.trim() || !company.trim()}
                onClick={async () => {
                  setCreating(true);
                  setError(null);
                  try {
                    if (!isNative()) {
                      // Browser mode — just complete onboarding
                      completeOnboarding({ defaultDomain: domain ?? "both", defaultEpsg: epsg });
                      return;
                    }
                    await invoke("create_account_cmd", {
                      name,
                      email,
                      company,
                      registrationNumber: regNumber.trim() || null,
                      phone: phone.trim() || null,
                    });
                    completeOnboarding({ defaultDomain: domain ?? "both", defaultEpsg: epsg });
                  } catch (err) {
                    setError(err instanceof Error ? err.message : String(err));
                  } finally {
                    setCreating(false);
                  }
                }}
                className="flex w-full items-center justify-center gap-2 rounded-md px-4 py-2.5 text-sm font-medium transition-colors disabled:cursor-not-allowed disabled:opacity-40"
                style={{
                  background: name && email && company ? colors.industrialOrange : colors.steelGray,
                  color: colors.navyBase,
                }}
              >
                {creating ? "Creating…" : "Create Account & Start"}
                {!creating && <CheckCircle2 className="h-4 w-4" />}
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

interface DomainCardProps {
  selected: boolean;
  onClick: () => void;
  accent: string;
  accentSecondary: string;
  icon: React.ReactNode;
  title: string;
  description: string;
}

function DomainCard({
  selected,
  onClick,
  accent,
  accentSecondary,
  icon,
  title,
  description,
}: DomainCardProps) {
  return (
    <button
      onClick={onClick}
      className={`group relative flex flex-col items-start rounded-lg border p-5 text-left transition-all ${
        selected
          ? "border-transparent bg-navy-elevated"
          : "border-navy-border bg-navy-panel hover:border-navy-border hover:bg-navy-elevated/50"
      }`}
      style={
        selected
          ? {
              boxShadow: `0 0 0 2px ${accent}, 0 8px 24px -8px ${accent}40`,
            }
          : undefined
      }
    >
      <div
        className="mb-3 flex h-12 w-12 items-center justify-center rounded-md"
        style={{
          background: selected
            ? `linear-gradient(135deg, ${accent}, ${accentSecondary})`
            : `${accent}15`,
          color: selected ? colors.navyBase : accent,
        }}
      >
        {icon}
      </div>
      <h3 className="text-base font-semibold text-white">{title}</h3>
      <p className="mt-1.5 text-xs leading-relaxed text-steel-light">
        {description}
      </p>
      {selected && (
        <div
          className="absolute right-3 top-3 h-2 w-2 rounded-full"
          style={{ background: accent }}
        />
      )}
    </button>
  );
}
