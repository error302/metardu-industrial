/**
 * First-Run Onboarding Screen
 * Two questions: which domain(s) + default CRS.
 * Selections configure default panels, shortcuts, color mode.
 * Skippable — power users can configure later in Settings.
 */

import { useState } from "react";
import { Mountain, Ship, ArrowRight, Search } from "lucide-react";
import {
  colors,
  domainAccent,
  APP_NAME,
  type DomainMode,
} from "@/lib/tokens";
import { useAppStore } from "@/stores/app-store";

// Common mining/marine CRS quick-picks
const CRS_QUICKPICKS = [
  { code: "EPSG:4326", label: "WGS 84 (geographic)" },
  { code: "EPSG:3857", label: "Web Mercator" },
  { code: "EPSG:28354", label: "MGA Zone 54 (Australia)" },
  { code: "EPSG:28355", label: "MGA Zone 55 (Australia)" },
  { code: "EPSG:28356", label: "MGA Zone 56 (Australia)" },
  { code: "EPSG:32733", label: "UTM Zone 33S" },
  { code: "EPSG:32734", label: "UTM Zone 34S" },
  { code: "EPSG:32633", label: "UTM Zone 33N" },
  { code: "EPSG:4269", label: "NAD83 (North America)" },
  { code: "EPSG:2154", label: "RGF93 / Lambert-93 (France)" },
];

export function OnboardingScreen() {
  const completeOnboarding = useAppStore((s) => s.completeOnboarding);
  const [domain, setDomain] = useState<DomainMode | null>(null);
  const [epsg, setEpsg] = useState("EPSG:4326");
  const [search, setSearch] = useState("");

  const filteredCrs = CRS_QUICKPICKS.filter(
    (c) =>
      c.code.toLowerCase().includes(search.toLowerCase()) ||
      c.label.toLowerCase().includes(search.toLowerCase()),
  );

  return (
    <div className="flex h-full w-full flex-col bg-navy-base">
      {/* Header */}
      <header className="flex h-12 items-center justify-between border-b border-navy-border px-6">
        <div className="flex items-center gap-3">
          <div
            className="flex h-7 w-7 items-center justify-center rounded font-bold"
            style={{ background: colors.industrialOrange, color: colors.navyBase }}
          >
            M
          </div>
          <span className="text-sm font-medium tracking-wide text-white">
            {APP_NAME}
          </span>
        </div>
        <button
          onClick={() =>
            completeOnboarding({ defaultDomain: "both", defaultEpsg: "EPSG:4326" })
          }
          className="text-xs text-steel-gray hover:text-white"
        >
          Skip onboarding →
        </button>
      </header>

      {/* Body */}
      <div className="flex-1 overflow-y-auto">
        <div className="mx-auto max-w-3xl px-8 py-10">
          <h1 className="text-2xl font-bold text-white">
            Welcome to {APP_NAME}
          </h1>
          <p className="mt-2 text-sm text-steel-light">
            Configure your default workspace. These choices shape your panels,
            keyboard shortcuts, and color mode. Switchable any time in Settings.
          </p>

          {/* Domain selection */}
          <section className="mt-10">
            <h2 className="mb-1 text-sm font-semibold uppercase tracking-wider text-steel-light">
              1 · Which surveys will you be running?
            </h2>
            <p className="mb-5 text-xs text-steel-gray">
              Select both if you serve mining and marine clients.
            </p>

            <div className="grid grid-cols-3 gap-4">
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
          <section className="mt-10">
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
          <div className="mt-10 flex items-center justify-between">
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
              onClick={() =>
                domain &&
                completeOnboarding({
                  defaultDomain: domain,
                  defaultEpsg: epsg,
                })
              }
              className="flex items-center gap-2 rounded-md px-4 py-2 text-sm font-medium transition-colors disabled:cursor-not-allowed disabled:opacity-40"
              style={{
                background: domain ? colors.industrialOrange : colors.steelGray,
                color: colors.navyBase,
              }}
            >
              Get started
              <ArrowRight className="h-4 w-4" />
            </button>
          </div>
        </div>
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
