/**
 * Settings Dialog — change default domain, EPSG, density.
 * Persists via Tauri IPC (save_settings) with localStorage fallback.
 */

import { useState } from "react";
import { Mountain, Ship, X, Save, RotateCcw } from "lucide-react";
import {
  colors,
  domainAccent,
  type DomainMode,
} from "@/lib/tokens";
import { useAppStore, type AppSettings } from "@/stores/app-store";
import { saveSettings } from "@/lib/tauri-ipc";

const EPSG_PRESETS = [
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

  if (!open) return null;

  const dirty =
    draft.defaultDomain !== settings.defaultDomain ||
    draft.defaultEpsg !== settings.defaultEpsg ||
    draft.density !== settings.density ||
    draft.reducedMotion !== settings.reducedMotion;

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
        className="w-full max-w-lg rounded-lg border border-navy-border bg-navy-panel shadow-2xl"
      >
        {/* Header */}
        <div className="flex items-center justify-between border-b border-navy-border px-5 py-3">
          <h2 className="text-sm font-semibold text-white">Settings</h2>
          <button
            onClick={onClose}
            className="rounded p-1 text-steel-gray hover:bg-navy-elevated hover:text-white"
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        {/* Body */}
        <div className="max-h-[60vh] overflow-y-auto p-5">
          {/* Domain */}
          <section className="mb-6">
            <h3 className="mb-2 text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
              Default Domain
            </h3>
            <div className="grid grid-cols-3 gap-2">
              <DomainPill
                selected={draft.defaultDomain === "mining"}
                onClick={() =>
                  setDraft({ ...draft, defaultDomain: "mining" })
                }
                accent={domainAccent.mining.primary}
                icon={<Mountain className="h-4 w-4" />}
                label="Mining"
              />
              <DomainPill
                selected={draft.defaultDomain === "marine"}
                onClick={() =>
                  setDraft({ ...draft, defaultDomain: "marine" })
                }
                accent={domainAccent.marine.primary}
                icon={<Ship className="h-4 w-4" />}
                label="Marine"
              />
              <DomainPill
                selected={draft.defaultDomain === "both"}
                onClick={() =>
                  setDraft({ ...draft, defaultDomain: "both" })
                }
                accent={domainAccent.both.primary}
                icon={
                  <div className="relative">
                    <Mountain className="h-4 w-4" />
                    <Ship className="absolute -bottom-0.5 -right-0.5 h-2.5 w-2.5" />
                  </div>
                }
                label="Both"
              />
            </div>
          </section>

          {/* CRS */}
          <section className="mb-6">
            <h3 className="mb-2 text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
              Default Coordinate System
            </h3>
            <select
              value={draft.defaultEpsg}
              onChange={(e) =>
                setDraft({ ...draft, defaultEpsg: e.target.value })
              }
              className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 font-mono text-sm text-white focus:border-industrial-orange focus:outline-none"
            >
              {EPSG_PRESETS.map((p) => (
                <option key={p.code} value={p.code}>
                  {p.code} — {p.label}
                </option>
              ))}
            </select>
            <p className="mt-1.5 text-[10px] text-steel-gray">
              Custom mine grids can be registered in Settings → CRS Library
              (Phase 1 feature).
            </p>
          </section>

          {/* UI Density */}
          <section className="mb-6">
            <h3 className="mb-2 text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
              UI Density
            </h3>
            <div className="grid grid-cols-2 gap-2">
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

          {/* Reduced motion */}
          <section>
            <label className="flex cursor-pointer items-center justify-between rounded-md border border-navy-border bg-navy-base px-3 py-2.5">
              <div>
                <div className="text-sm text-white">Reduced motion</div>
                <div className="text-[10px] text-steel-gray">
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
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between border-t border-navy-border px-5 py-3">
          <button
            onClick={reset}
            disabled={!dirty}
            className="flex items-center gap-1.5 rounded-md px-3 py-1.5 text-xs text-steel-light transition-colors hover:bg-navy-elevated disabled:opacity-40"
          >
            <RotateCcw className="h-3 w-3" />
            Reset
          </button>
          <div className="flex items-center gap-2">
            {saved && (
              <span
                className="text-[10px] font-medium"
                style={{ color: colors.pass }}
              >
                Saved ✓
              </span>
            )}
            <button
              onClick={apply}
              disabled={!dirty || saving}
              className="flex items-center gap-1.5 rounded-md px-4 py-1.5 text-xs font-medium transition-colors disabled:opacity-40"
              style={{
                background: dirty
                  ? colors.industrialOrange
                  : colors.steelGray,
                color: colors.navyBase,
              }}
            >
              <Save className="h-3 w-3" />
              {saving ? "Saving…" : "Save"}
            </button>
          </div>
        </div>
      </div>
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
      className="flex flex-col items-center gap-1.5 rounded-md border p-3 transition-all"
      style={{
        borderColor: selected ? accent : colors.navyBorder,
        background: selected ? `${accent}15` : colors.navyBase,
      }}
    >
      <div
        className="flex h-8 w-8 items-center justify-center rounded-md"
        style={{
          background: selected ? `${accent}30` : `${colors.navyElevated}`,
          color: selected ? accent : colors.steelLight,
        }}
      >
        {icon}
      </div>
      <span
        className="text-xs font-medium"
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
      className="rounded-md border p-3 text-left transition-all"
      style={{
        borderColor: selected ? colors.industrialOrange : colors.navyBorder,
        background: selected ? `${colors.industrialOrange}10` : colors.navyBase,
      }}
    >
      <div
        className="text-sm font-medium"
        style={{ color: selected ? colors.white : colors.steelLight }}
      >
        {label}
      </div>
      <div className="mt-0.5 text-[10px] text-steel-gray">{description}</div>
    </button>
  );
}

// Suppress unused import warning for DomainMode — kept for future use
export type { DomainMode };
