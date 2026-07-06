/**
 * Safety Inspection Report — Sprint 10 Mining Field Tool #4.
 *
 * Hazard register with severity, risk level, and recommended actions,
 * rendered as a plain-text regulator-ready inspection report.
 *
 * Workflow:
 *   1. Enter inspection metadata (date, inspector, area, overall risk)
 *   2. Add hazards (type, location, description, severity, status)
 *   3. Add recommended actions
 *   4. Click Generate → preview + copy report text
 */

import { useState } from "react";
import { X, ShieldAlert, Loader2, Plus, Trash2, Copy } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { colors } from "@/lib/tokens";
import { isNative } from "@/lib/tauri-ipc";
import { useEscapeKey } from "@/lib/use-escape-key";

type HazardType =
  | "wall_instability" | "rockfall" | "water_inflow" | "equipment"
  | "blast_misfire" | "slope_failure" | "subsidence" | "other";

type RiskLevel = "low" | "moderate" | "high" | "critical";
type HazardStatus = "open" | "mitigated" | "resolved";

interface Hazard {
  hazard_type: HazardType;
  location: [number, number, number];
  description: string;
  severity: number; // 1-5
  status: HazardStatus;
}

interface SafetyInspection {
  date: string;
  inspector: string;
  area: string;
  hazards: Hazard[];
  risk_level: RiskLevel;
  recommendations: string[];
}

interface Props {
  open: boolean;
  onClose: () => void;
}

const HAZARD_TYPES: { value: HazardType; label: string }[] = [
  { value: "wall_instability", label: "Wall Instability" },
  { value: "rockfall", label: "Rockfall" },
  { value: "water_inflow", label: "Water Inflow" },
  { value: "equipment", label: "Equipment" },
  { value: "blast_misfire", label: "Blast Misfire" },
  { value: "slope_failure", label: "Slope Failure" },
  { value: "subsidence", label: "Subsidence" },
  { value: "other", label: "Other" },
];

const RISK_LEVELS: { value: RiskLevel; label: string; color: string }[] = [
  { value: "low", label: "Low", color: colors.pass },
  { value: "moderate", label: "Moderate", color: colors.warn },
  { value: "high", label: "High", color: colors.fail },
  { value: "critical", label: "Critical", color: colors.failDim },
];

const STATUSES: { value: HazardStatus; label: string }[] = [
  { value: "open", label: "Open" },
  { value: "mitigated", label: "Mitigated" },
  { value: "resolved", label: "Resolved" },
];

export function SafetyReportDialog({ open, onClose }: Props) {
  const [inspection, setInspection] = useState<SafetyInspection>({
    date: new Date().toISOString().slice(0, 10),
    inspector: "",
    area: "",
    hazards: [
      { hazard_type: "rockfall", location: [1024.5, 2050.3, 1050.0], description: "Loose boulder on bench face", severity: 4, status: "open" },
    ],
    risk_level: "moderate",
    recommendations: ["Scale bench face before resuming operations", "Install catch fence on berm"],
  });
  const [report, setReport] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  useEscapeKey(onClose, open);
  if (!open) return null;

  function addHazard() {
    setInspection({
      ...inspection,
      hazards: [
        ...inspection.hazards,
        { hazard_type: "other", location: [0, 0, 0], description: "", severity: 3, status: "open" },
      ],
    });
  }

  function removeHazard(i: number) {
    setInspection({
      ...inspection,
      hazards: inspection.hazards.filter((_, idx) => idx !== i),
    });
  }

  function updateHazard(i: number, patch: Partial<Hazard>) {
    setInspection({
      ...inspection,
      hazards: inspection.hazards.map((h, idx) => (idx === i ? { ...h, ...patch } : h)),
    });
  }

  async function handleGenerate() {
    setLoading(true);
    setError(null);
    setReport(null);
    try {
      if (!isNative()) {
        setError("Browser mode — report generation requires the native Tauri shell");
        return;
      }
      const r = await invoke<string>("generate_safety_report_cmd", { inspection });
      setReport(r);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }

  function copyReport() {
    if (!report) return;
    navigator.clipboard.writeText(report);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  }

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
      onClick={onClose}
    >
      <div
        onClick={(e) => e.stopPropagation()}
        className="flex max-h-[90vh] w-full max-w-4xl flex-col rounded-lg border border-navy-border bg-navy-panel shadow-2xl"
      >
        <div className="flex items-center justify-between border-b border-navy-border px-5 py-3">
          <h2 className="flex items-center gap-2 text-sm font-semibold text-white">
            <ShieldAlert className="h-4 w-4" style={{ color: colors.fail }} />
            Safety Inspection Report
          </h2>
          <button onClick={onClose} className="rounded p-1 text-steel-gray hover:bg-navy-elevated hover:text-white">
            <X className="h-4 w-4" />
          </button>
        </div>

        <div className="flex-1 overflow-y-auto p-5 grid grid-cols-2 gap-5">
          {/* Left: form */}
          <div className="space-y-3">
            <div className="grid grid-cols-2 gap-2">
              <Field label="Date" value={inspection.date} onChange={(v) => setInspection({ ...inspection, date: v })} />
              <Field label="Inspector" value={inspection.inspector} onChange={(v) => setInspection({ ...inspection, inspector: v })} />
            </div>
            <Field label="Area Inspected" value={inspection.area} onChange={(v) => setInspection({ ...inspection, area: v })} placeholder="Pit A — Bench 1050" />

            <div>
              <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">Overall Risk Level</label>
              <div className="flex gap-1">
                {RISK_LEVELS.map((r) => (
                  <button
                    key={r.value}
                    onClick={() => setInspection({ ...inspection, risk_level: r.value })}
                    className={`flex-1 rounded-md px-2 py-1.5 text-[10px] font-medium ${inspection.risk_level === r.value ? "text-navy-base" : "text-steel-gray"}`}
                    style={{ background: inspection.risk_level === r.value ? r.color : colors.navyBase, border: `1px solid ${r.color}40` }}
                  >
                    {r.label}
                  </button>
                ))}
              </div>
            </div>

            {/* Hazards */}
            <div>
              <div className="mb-1.5 flex items-center justify-between">
                <label className="text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
                  Hazards ({inspection.hazards.length})
                </label>
                <button
                  onClick={addHazard}
                  className="flex items-center gap-1 rounded px-2 py-0.5 text-[10px] font-medium"
                  style={{ background: colors.fail, color: colors.navyBase }}
                >
                  <Plus className="h-3 w-3" /> Add
                </button>
              </div>
              <div className="max-h-64 space-y-1.5 overflow-y-auto">
                {inspection.hazards.map((h, i) => (
                  <div key={i} className="rounded-md border border-navy-border bg-navy-base p-2">
                    <div className="mb-1.5 grid grid-cols-3 gap-1.5">
                      <select
                        value={h.hazard_type}
                        onChange={(e) => updateHazard(i, { hazard_type: e.target.value as HazardType })}
                        className="rounded border border-navy-border bg-navy-base px-1 py-0.5 text-[10px] text-steel-light"
                      >
                        {HAZARD_TYPES.map((t) => (
                          <option key={t.value} value={t.value}>{t.label}</option>
                        ))}
                      </select>
                      <select
                        value={h.status}
                        onChange={(e) => updateHazard(i, { status: e.target.value as HazardStatus })}
                        className="rounded border border-navy-border bg-navy-base px-1 py-0.5 text-[10px] text-steel-light"
                      >
                        {STATUSES.map((s) => (
                          <option key={s.value} value={s.value}>{s.label}</option>
                        ))}
                      </select>
                      <div className="flex items-center gap-1">
                        <span className="text-[9px] text-steel-gray">Sev:</span>
                        <input
                          type="number"
                          min={1}
                          max={5}
                          value={h.severity}
                          onChange={(e) => updateHazard(i, { severity: Math.max(1, Math.min(5, parseInt(e.target.value) || 1)) })}
                          className="w-10 rounded border border-navy-border bg-navy-base px-1 py-0.5 text-right text-[10px] text-white"
                        />
                        <button
                          onClick={() => removeHazard(i)}
                          className="ml-auto rounded p-0.5 text-steel-gray hover:bg-fail/20 hover:text-fail"
                        >
                          <Trash2 className="h-3 w-3" />
                        </button>
                      </div>
                    </div>
                    <input
                      value={h.description}
                      onChange={(e) => updateHazard(i, { description: e.target.value })}
                      placeholder="Description"
                      className="mb-1.5 w-full rounded border border-navy-border bg-navy-base px-2 py-0.5 text-[10px] text-white"
                    />
                    <div className="grid grid-cols-3 gap-1.5">
                      {(["E", "N", "Z"] as const).map((lab, j) => (
                        <div key={lab}>
                          <label className="text-[8px] text-steel-gray">{lab}</label>
                          <input
                            type="number"
                            value={h.location[j]}
                            step="0.1"
                            onChange={(e) => {
                              const loc = [...h.location] as [number, number, number];
                              loc[j] = parseFloat(e.target.value) || 0;
                              updateHazard(i, { location: loc });
                            }}
                            className="w-full rounded border border-navy-border bg-navy-base px-1 py-0.5 text-right text-[10px] font-mono text-white"
                          />
                        </div>
                      ))}
                    </div>
                  </div>
                ))}
              </div>
            </div>

            {/* Recommendations */}
            <div>
              <div className="mb-1.5 flex items-center justify-between">
                <label className="text-[10px] font-semibold uppercase tracking-wider text-steel-gray">Recommendations</label>
                <button
                  onClick={() => setInspection({ ...inspection, recommendations: [...inspection.recommendations, ""] })}
                  className="flex items-center gap-1 rounded px-2 py-0.5 text-[10px] font-medium"
                  style={{ background: colors.steelLight, color: colors.navyBase }}
                >
                  <Plus className="h-3 w-3" /> Add
                </button>
              </div>
              <div className="space-y-1">
                {inspection.recommendations.map((r, i) => (
                  <div key={i} className="flex gap-1">
                    <span className="mt-1 text-[10px] text-steel-gray">{i + 1}.</span>
                    <input
                      value={r}
                      onChange={(e) => setInspection({
                        ...inspection,
                        recommendations: inspection.recommendations.map((x, idx) => (idx === i ? e.target.value : x)),
                      })}
                      className="flex-1 rounded border border-navy-border bg-navy-base px-2 py-0.5 text-[10px] text-white"
                    />
                    <button
                      onClick={() => setInspection({ ...inspection, recommendations: inspection.recommendations.filter((_, idx) => idx !== i) })}
                      className="rounded p-0.5 text-steel-gray hover:bg-fail/20 hover:text-fail"
                    >
                      <Trash2 className="h-3 w-3" />
                    </button>
                  </div>
                ))}
              </div>
            </div>

            {error && (
              <div className="rounded-md border p-2 text-[10px]" style={{ borderColor: `${colors.fail}40`, background: `${colors.fail}10`, color: colors.fail }}>
                {error}
              </div>
            )}
          </div>

          {/* Right: report preview */}
          <div className="flex flex-col">
            <div className="mb-2 flex items-center justify-between">
              <label className="text-[10px] font-semibold uppercase tracking-wider text-steel-gray">Report Preview</label>
              {report && (
                <button
                  onClick={copyReport}
                  className="flex items-center gap-1 rounded px-2 py-0.5 text-[10px] font-medium"
                  style={{ background: colors.steelLight, color: colors.navyBase }}
                >
                  <Copy className="h-3 w-3" /> {copied ? "Copied!" : "Copy"}
                </button>
              )}
            </div>
            <pre className="flex-1 overflow-auto rounded-md border border-navy-border bg-navy-base p-3 font-mono text-[10px] leading-relaxed text-steel-light">
{report || "Click \"Generate Report\" to produce the inspection text.\n\nFill in inspector name and area, add hazards and recommendations, then click Generate."}
            </pre>
          </div>
        </div>

        <div className="flex items-center justify-between border-t border-navy-border px-5 py-3">
          <div className="text-[10px] text-steel-gray">
            Severity 1=low · 5=critical · Status open / mitigated / resolved
          </div>
          <div className="flex gap-2">
            <button
              onClick={onClose}
              className="rounded-md px-4 py-1.5 text-xs font-medium"
              style={{ background: colors.steelGray, color: colors.navyBase }}
            >
              Close
            </button>
            <button
              onClick={handleGenerate}
              disabled={loading}
              className="flex items-center gap-1.5 rounded-md px-4 py-1.5 text-xs font-medium disabled:opacity-40"
              style={{ background: colors.fail, color: colors.white }}
            >
              {loading ? <Loader2 className="h-3 w-3 animate-spin" /> : <ShieldAlert className="h-3 w-3" />}
              {loading ? "Generating…" : "Generate Report"}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

function Field({ label, value, onChange, placeholder }: { label: string; value: string; onChange: (v: string) => void; placeholder?: string }) {
  return (
    <div>
      <label className="mb-0.5 block text-[9px] uppercase tracking-wider text-steel-gray">{label}</label>
      <input
        type="text"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        className="w-full rounded border border-navy-border bg-navy-base px-2 py-1 text-xs text-white focus:border-fail focus:outline-none"
      />
    </div>
  );
}
