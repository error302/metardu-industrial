import { useEscapeKey } from "@/lib/use-escape-key";
/**
 * Settings Dialog — workspace defaults, CRS library, accessibility, about.
 *
 * Robust layout: max-w-2xl, sectioned with dividers, larger touch targets,
 * and a prominent Save action. Settings persist via Tauri IPC (save_settings)
 * with a localStorage fallback handled in the app store.
 */

import { useMemo, useState } from "react";
import {
  Mountain,
  Ship,
  X,
  Save,
  RotateCcw,
  Search,
  Library,
  Info,
  Cpu,
} from "lucide-react";
import {
  colors,
  domainAccent,
  APP_VERSION,
  APP_BUILD,
  type DomainMode,
} from "@/lib/tokens";
import { BrandLogoMark } from "@/components/brand-logo";
import { useAppStore, type AppSettings } from "@/stores/app-store";
import { saveSettings } from "@/lib/tauri-ipc";

/**
 * Common EPSG codes — same set used by the onboarding screen's CRS_QUICKPICKS.
 * Surfaced here as a searchable grid so users can pick a default CRS without
 * having to type the code.
 */
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

interface Props {
  open: boolean;
  onClose: () => void;
}

export function SettingsDialog({ open, onClose }: Props) {
  const settings = useAppStore((s) => s.settings);
  const updateSettings = useAppStore((s) => s.updateSettings);
  const setActiveDomain = useAppStore((s) => s.setActiveDomain);

  const [draft, setDraft] = useState<AppSettings>(settings);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [crsSearch, setCrsSearch] = useState("");

  const filteredCrs = useMemo(() => {
    const q = crsSearch.trim().toLowerCase();
    if (!q) return CRS_QUICKPICKS;
    return CRS_QUICKPICKS.filter(
      (c) =>
        c.code.toLowerCase().includes(q) ||
        c.label.toLowerCase().includes(q),
    );
  }, [crsSearch]);

  useEscapeKey(onClose, open);
  if (!open) return null;

  const dirty =
    draft.defaultDomain !== settings.defaultDomain ||
    draft.defaultEpsg !== settings.defaultEpsg ||
    draft.density !== settings.density ||
    draft.reducedMotion !== settings.reducedMotion ||
    draft.theme !== settings.theme;

  function apply() {
    setSaving(true);
    updateSettings(draft);
    setActiveDomain(draft.defaultDomain);
    void saveSettings({
      defaultDomain: draft.defaultDomain,
      defaultEpsg: draft.defaultEpsg,
      density: draft.density,
      reducedMotion: draft.reducedMotion,
    }).finally(() => {
      setSaving(false);
      setSaved(true);
      setTimeout(() => setSaved(false), 1500);
    });
  }

  function reset() {
    setDraft(settings);
  }

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
      onClick={onClose}
    >
      <div
        onClick={(e) => e.stopPropagation()}
        className="flex max-h-[88vh] w-full max-w-2xl flex-col rounded-lg border border-navy-border bg-navy-panel shadow-2xl"
      >
        {/* Header */}
        <div className="flex items-center justify-between border-b border-navy-border px-6 py-4">
          <div className="flex items-center gap-2.5">
            <SettingsGlyph />
            <h2 className="text-base font-semibold text-white">Settings</h2>
          </div>
          <button
            onClick={onClose}
            className="rounded p-1.5 text-steel-light transition-colors hover:bg-navy-elevated hover:text-white"
            aria-label="Close settings"
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        {/* Body */}
        <div className="flex-1 overflow-y-auto p-6">
          {/* ── Default Workspace ── */}
          <SectionHeader title="Default Workspace" />
          <section className="mb-7">
            <h3 className="mb-2.5 text-[11px] font-medium uppercase tracking-wider text-steel-light">
              Domain
            </h3>
            <div className="grid grid-cols-3 gap-3">
              <DomainPill
                selected={draft.defaultDomain === "mining"}
                onClick={() =>
                  setDraft({ ...draft, defaultDomain: "mining" })
                }
                accent={domainAccent.mining.primary}
                icon={<Mountain className="h-5 w-5" />}
                label="Mining"
              />
              <DomainPill
                selected={draft.defaultDomain === "marine"}
                onClick={() =>
                  setDraft({ ...draft, defaultDomain: "marine" })
                }
                accent={domainAccent.marine.primary}
                icon={<Ship className="h-5 w-5" />}
                label="Marine"
              />
              <DomainPill
                selected={draft.defaultDomain === "both"}
                onClick={() => setDraft({ ...draft, defaultDomain: "both" })}
                accent={domainAccent.both.primary}
                icon={
                  <div className="relative">
                    <Mountain className="h-5 w-5" />
                    <Ship className="absolute -bottom-0.5 -right-0.5 h-3 w-3" />
                  </div>
                }
                label="Both"
              />
            </div>

            <h3 className="mb-2.5 mt-5 text-[11px] font-medium uppercase tracking-wider text-steel-light">
              UI Density
            </h3>
            <div className="grid grid-cols-2 gap-3">
              <DensityPill
                selected={draft.density === "comfortable"}
                onClick={() =>
                  setDraft({ ...draft, density: "comfortable" })
                }
                label="Comfortable"
                description="16px row height"
              />
              <DensityPill
                selected={draft.density === "compact"}
                onClick={() => setDraft({ ...draft, density: "compact" })}
                label="Compact"
                description="12px row height"
              />
            </div>
          </section>

          {/* ── Coordinate System ── */}
          <SectionHeader title="Coordinate System" />
          <section className="mb-7">
            <label
              className="mb-2 block text-[11px] font-medium uppercase tracking-wider text-steel-light"
              htmlFor="settings-default-epsg"
            >
              Default CRS
            </label>
            <select
              id="settings-default-epsg"
              value={draft.defaultEpsg}
              onChange={(e) =>
                setDraft({ ...draft, defaultEpsg: e.target.value })
              }
              className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2.5 font-mono text-[14px] text-white transition-colors focus:border-industrial-orange focus:outline-none"
            >
              {CRS_QUICKPICKS.map((p) => (
                <option key={p.code} value={p.code}>
                  {p.code} — {p.label}
                </option>
              ))}
            </select>
            <p className="mt-2 text-[11px] leading-relaxed text-steel-light">
              Pick a common CRS above, or browse the library below. Custom mine
              grids can be registered via proj4 strings in a future release.
            </p>
          </section>

          {/* ── CRS Library ── */}
          <SectionHeader title="CRS Library" icon={<Library className="h-3.5 w-3.5" />} />
          <section className="mb-7">
            <div className="relative mb-3">
              <Search className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-steel-gray" />
              <input
                type="text"
                value={crsSearch}
                onChange={(e) => setCrsSearch(e.target.value)}
                placeholder="Search EPSG codes or descriptions…"
                className="w-full rounded-md border border-navy-border bg-navy-base py-2.5 pl-9 pr-3 text-[14px] text-white placeholder:text-steel-gray focus:border-industrial-orange focus:outline-none"
              />
            </div>
            <div className="grid max-h-56 grid-cols-2 gap-2 overflow-y-auto pr-1">
              {filteredCrs.map((c) => {
                const selected = draft.defaultEpsg === c.code;
                return (
                  <button
                    key={c.code}
                    onClick={() =>
                      setDraft({ ...draft, defaultEpsg: c.code })
                    }
                    className="rounded-md border p-3 text-left transition-all"
                    style={{
                      borderColor: selected
                        ? colors.industrialOrange
                        : colors.navyBorder,
                      background: selected
                        ? `${colors.industrialOrange}15`
                        : colors.navyBase,
                    }}
                  >
                    <div
                      className="font-mono text-[13px] font-semibold"
                      style={{
                        color: selected
                          ? colors.industrialOrange
                          : colors.white,
                      }}
                    >
                      {c.code}
                    </div>
                    <div className="mt-0.5 text-[11px] text-steel-light">
                      {c.label}
                    </div>
                  </button>
                );
              })}
              {filteredCrs.length === 0 && (
                <div className="col-span-2 rounded-md border border-dashed border-navy-border p-4 text-center text-[12px] text-steel-gray">
                  No CRS matches “{crsSearch}”.
                </div>
              )}
            </div>
          </section>

          {/* ── Accessibility ── */}
          <SectionHeader title="Accessibility & Theme" />
          <section className="mb-7">
            {/* Theme toggle */}
            <div className="mb-3">
              <div className="mb-2 text-[12px] font-semibold text-steel-light">
                Display theme
              </div>
              <div className="grid grid-cols-2 gap-2">
                <button
                  onClick={() => setDraft({ ...draft, theme: "dark" })}
                  className="rounded-md border p-3 text-left transition-colors"
                  style={{
                    borderColor: draft.theme === "dark" ? colors.industrialOrange : colors.navyBorder,
                    background: draft.theme === "dark" ? `${colors.industrialOrange}10` : "var(--color-navy-base)",
                  }}
                >
                  <div className="text-[14px] font-semibold" style={{ color: draft.theme === "dark" ? colors.industrialOrange : "var(--color-steel-light)" }}>
                    Dark (Cabin)
                  </div>
                  <div className="mt-0.5 text-[11px] text-steel-gray">
                    Navy background — for low-light survey cabins & control rooms
                  </div>
                </button>
                <button
                  onClick={() => setDraft({ ...draft, theme: "light" })}
                  className="rounded-md border p-3 text-left transition-colors"
                  style={{
                    borderColor: draft.theme === "light" ? colors.industrialOrange : colors.navyBorder,
                    background: draft.theme === "light" ? `${colors.industrialOrange}10` : "var(--color-navy-base)",
                  }}
                >
                  <div className="text-[14px] font-semibold" style={{ color: draft.theme === "light" ? colors.industrialOrange : "var(--color-steel-light)" }}>
                    Daylight (Field)
                  </div>
                  <div className="mt-0.5 text-[11px] text-steel-gray">
                    White high-contrast — for outdoor use in direct sunlight
                  </div>
                </button>
              </div>
            </div>

            {/* Reduced motion */}
            <label className="flex cursor-pointer items-center justify-between rounded-md border border-navy-border bg-navy-base px-3 py-3">
              <div>
                <div className="text-[14px] font-medium text-white">
                  Reduced motion
                </div>
                <div className="mt-0.5 text-[11px] text-steel-light">
                  Disable splash animations and transitions
                </div>
              </div>
              <input
                type="checkbox"
                checked={draft.reducedMotion}
                onChange={(e) =>
                  setDraft({ ...draft, reducedMotion: e.target.checked })
                }
                className="h-4 w-4"
                style={{ accentColor: colors.industrialOrange }}
              />
            </label>
          </section>

          {/* ── About ── */}
          <SectionHeader title="About" icon={<Info className="h-3.5 w-3.5" />} />
          <section>
            {/* Brand logo + name */}
            <div className="mb-3 flex items-center gap-3 rounded-md border border-navy-border bg-navy-base p-3">
              <BrandLogoMark size={48} />
              <div>
                <div className="text-sm font-bold text-white">
                  Meta<span style={{ color: colors.industrialOrange }}>RDU</span> Industrial
                </div>
                <div className="text-[10px] tracking-[0.2em] font-semibold" style={{ color: colors.industrialOrange }}>
                  MINING & MARINE SURVEYS
                </div>
              </div>
            </div>
            <div className="grid grid-cols-2 gap-3">
              <AboutTile label="Version" value={`v${APP_VERSION}`} mono />
              <AboutTile label="Build Date" value={APP_BUILD} mono />
              <div className="col-span-2 rounded-md border border-navy-border bg-navy-base p-3">
                <div className="flex items-center gap-1.5 text-[10px] font-medium uppercase tracking-wider text-steel-gray">
                  <Cpu className="h-3 w-3" />
                  Tech Stack
                </div>
                <div className="mt-1 text-[13px] text-white">
                  Tauri 2.0 · React 19 · TypeScript · Tailwind CSS 4
                </div>
              </div>
            </div>
          </section>
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between border-t border-navy-border px-6 py-4">
          <button
            onClick={reset}
            disabled={!dirty}
            className="flex items-center gap-1.5 rounded-md px-3 py-2 text-[13px] text-steel-light transition-colors hover:bg-navy-elevated hover:text-white disabled:cursor-not-allowed disabled:opacity-40"
          >
            <RotateCcw className="h-3.5 w-3.5" />
            Reset
          </button>
          <div className="flex items-center gap-3">
            {saved && (
              <span
                className="text-[12px] font-semibold"
                style={{ color: colors.pass }}
              >
                Saved ✓
              </span>
            )}
            <button
              onClick={apply}
              disabled={!dirty || saving}
              className="flex items-center gap-2 rounded-md px-5 py-2.5 text-[13px] font-bold shadow-lg transition-all hover:brightness-110 disabled:cursor-not-allowed disabled:opacity-40 disabled:shadow-none"
              style={{
                background: dirty ? colors.industrialOrange : colors.steelGray,
                color: colors.navyBase,
                boxShadow: dirty
                  ? `0 4px 14px ${colors.industrialOrange}40`
                  : "none",
              }}
            >
              <Save className="h-4 w-4" />
              {saving ? "Saving…" : "Save Changes"}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

/* ──────────────────────────────────────────────────────────── */

function SettingsGlyph() {
  return (
    <div
      className="flex h-7 w-7 items-center justify-center rounded"
      style={{ background: colors.industrialOrange, color: colors.navyBase }}
      aria-hidden
    >
      <svg
        width="16"
        height="16"
        viewBox="0 0 120 120"
        fill="none"
        xmlns="http://www.w3.org/2000/svg"
      >
        <circle
          cx="60"
          cy="60"
          r="54"
          stroke="currentColor"
          strokeWidth="10"
          fill="none"
        />
        <text
          x="60"
          y="82"
          textAnchor="middle"
          fontSize="62"
          fontWeight="900"
          fontFamily="Inter, system-ui, sans-serif"
          fill="currentColor"
        >
          M
        </text>
      </svg>
    </div>
  );
}

function SectionHeader({
  title,
  icon,
}: {
  title: string;
  icon?: React.ReactNode;
}) {
  return (
    <div className="mb-3 flex items-center gap-2.5">
      <h2 className="flex items-center gap-1.5 text-[11px] font-semibold uppercase tracking-[0.18em] text-steel-light whitespace-nowrap">
        {icon}
        {title}
      </h2>
      <div className="h-px flex-1 bg-navy-border" />
    </div>
  );
}

interface DomainPillProps {
  selected: boolean;
  onClick: () => void;
  accent: string;
  icon: React.ReactNode;
  label: string;
}

function DomainPill({
  selected,
  onClick,
  accent,
  icon,
  label,
}: DomainPillProps) {
  return (
    <button
      onClick={onClick}
      className="flex flex-col items-center gap-2 rounded-md border p-4 transition-all"
      style={{
        borderColor: selected ? accent : colors.navyBorder,
        background: selected ? `${accent}15` : colors.navyBase,
      }}
    >
      <div
        className="flex h-10 w-10 items-center justify-center rounded-md"
        style={{
          background: selected ? `${accent}30` : `${colors.navyElevated}`,
          color: selected ? accent : colors.steelLight,
        }}
      >
        {icon}
      </div>
      <span
        className="text-[13px] font-semibold"
        style={{ color: selected ? colors.white : colors.steelLight }}
      >
        {label}
      </span>
    </button>
  );
}

interface DensityPillProps {
  selected: boolean;
  onClick: () => void;
  label: string;
  description: string;
}

function DensityPill({
  selected,
  onClick,
  label,
  description,
}: DensityPillProps) {
  return (
    <button
      onClick={onClick}
      className="rounded-md border p-4 text-left transition-all"
      style={{
        borderColor: selected ? colors.industrialOrange : colors.navyBorder,
        background: selected
          ? `${colors.industrialOrange}10`
          : colors.navyBase,
      }}
    >
      <div
        className="text-[14px] font-semibold"
        style={{ color: selected ? colors.white : colors.steelLight }}
      >
        {label}
      </div>
      <div className="mt-1 text-[11px] text-steel-light">{description}</div>
    </button>
  );
}

function AboutTile({
  label,
  value,
  mono = false,
}: {
  label: string;
  value: string;
  mono?: boolean;
}) {
  return (
    <div className="rounded-md border border-navy-border bg-navy-base p-3">
      <div className="text-[10px] font-medium uppercase tracking-wider text-steel-gray">
        {label}
      </div>
      <div
        className={`mt-1 text-[14px] text-white ${mono ? "font-mono" : ""}`}
      >
        {value}
      </div>
    </div>
  );
}

// Suppress unused import warning for DomainMode — kept for future use
export type { DomainMode };
