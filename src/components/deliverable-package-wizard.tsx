/**
 * Survey Deliverable Package Wizard — Sprint 5 Revenue Feature #7.
 *
 * Marine surveyors spend 4-6 hours per delivery manually assembling
 * GeoTIFF + S-57 + S-44 PDF + metadata XML + trackplot + tide log.
 * This wizard compresses that to 30 seconds.
 *
 * Workflow:
 *   1. Configure survey metadata (vessel, sonar, area, date, etc.)
 *   2. Add source files (GeoTIFF, S-57 .000, S-44 PDF, etc.)
 *   3. Set output ZIP path + project name
 *   4. Generate ZIP with manifest + ISO 19115 metadata XML
 *   5. Review bundled files + warnings, open ZIP location
 *
 * Revenue: $3,000-5,000/seat — every marine survey delivery needs this.
 */

import { useState } from "react";
import {
  X, ArrowRight, ArrowLeft, FileText, Loader2, CheckCircle2,
  Package, Download, Plus, Trash2, AlertTriangle,
} from "lucide-react";
import { colors } from "@/lib/tokens";
import {
  generateDeliverablePackage,
  type DeliverablePackageResult,
  type DeliverableFileType,
  type DeliverableSource,
} from "@/lib/tauri-ipc";
import { useSurveyStore } from "@/stores/survey-store";

interface Props {
  open: boolean;
  onClose: () => void;
}

type Step = 1 | 2 | 3 | 4 | 5;
const STEP_LABELS = ["Metadata", "Files", "Generate", "Review", "Done"];

const FILE_TYPES: { value: DeliverableFileType; label: string }[] = [
  { value: "geotiff", label: "GeoTIFF (bathymetric surface)" },
  { value: "s57", label: "S-57 .000 (ENC export)" },
  { value: "s44_pdf", label: "S-44 Compliance PDF" },
  { value: "metadata_xml", label: "ISO 19115 Metadata XML" },
  { value: "track_plot", label: "Track Plot PDF" },
  { value: "tide_log", label: "Tide Log CSV" },
  { value: "screenshot", label: "Map Screenshot PNG" },
  { value: "other", label: "Other" },
];

export function DeliverablePackageWizard({ open, onClose }: Props) {
  const files = useSurveyStore((s) => s.files);

  // Step 1: Metadata
  const [vessel, setVessel] = useState("");
  const [sonar, setSonar] = useState("");
  const [surveyArea, setSurveyArea] = useState("");
  const [surveyDate, setSurveyDate] = useState(new Date().toISOString().slice(0, 10));
  const [epsg, setEpsg] = useState("4326");
  const [clientName, setClientName] = useState("");
  const [surveyorName, setSurveyorName] = useState("");

  // Step 2: Source files
  const [sources, setSources] = useState<DeliverableSource[]>([
    { description: "", path: "", fileType: "geotiff" },
  ]);

  // Step 3: Output
  const [projectName, setProjectName] = useState("");
  const [outputPath, setOutputPath] = useState("/tmp/survey_deliverable.zip");

  const [step, setStep] = useState<Step>(1);
  const [generating, setGenerating] = useState(false);
  const [result, setResult] = useState<DeliverablePackageResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  if (!open) return null;

  const metadataValid = vessel && sonar && surveyArea && surveyDate && epsg && clientName && surveyorName;
  const sourcesValid = sources.filter((s) => s.path).length >= 1;
  const canNext =
    step === 1 ? !!metadataValid :
    step === 2 ? sourcesValid :
    step === 3 ? result !== null :
    step === 4 ? true :
    false;

  function addSource() {
    setSources([...sources, { description: "", path: "", fileType: "other" }]);
  }
  function removeSource(idx: number) {
    setSources(sources.filter((_, i) => i !== idx));
  }
  function updateSource(idx: number, field: keyof DeliverableSource, value: string) {
    setSources(sources.map((s, i) => (i === idx ? { ...s, [field]: value } : s)));
  }

  async function handleGenerate() {
    setGenerating(true);
    setError(null);
    setResult(null);
    try {
      const validSources = sources.filter((s) => s.path);
      const r = await generateDeliverablePackage({
        outputPath,
        projectName: projectName || "survey_deliverable",
        metadata: {
          vessel,
          sonar,
          surveyArea,
          surveyDate,
          epsg,
          clientName,
          surveyorName,
        },
        sources: validSources,
        mapScreenshotB64: undefined, // Frontend could capture from OL canvas if needed
      });
      if (r) {
        setResult(r);
        setStep(4);
      } else {
        setError("Browser mode — package generation requires the native Tauri shell");
      }
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setGenerating(false);
    }
  }

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
      onClick={onClose}
    >
      <div
        onClick={(e) => e.stopPropagation()}
        className="flex max-h-[88vh] w-full max-w-3xl flex-col rounded-lg border border-navy-border bg-navy-panel shadow-2xl"
      >
        {/* Header */}
        <div className="flex items-center justify-between border-b border-navy-border px-5 py-3">
          <h2 className="flex items-center gap-2 text-sm font-semibold text-white">
            <Package className="h-4 w-4" style={{ color: "#6366F1" }} />
            Survey Deliverable Package Wizard
          </h2>
          <button onClick={onClose} className="rounded p-1 text-steel-gray hover:bg-navy-elevated hover:text-white">
            <X className="h-4 w-4" />
          </button>
        </div>

        {/* Step indicator */}
        <div className="flex border-b border-navy-border px-5 py-2">
          {STEP_LABELS.map((label, i) => (
            <div key={i} className="flex items-center gap-1.5 px-2">
              <div
                className="flex h-5 w-5 items-center justify-center rounded-full text-[9px] font-bold"
                style={{
                  background: step > i + 1 ? colors.pass : step === i + 1 ? "#6366F1" : colors.navyBorder,
                  color: step >= i + 1 ? colors.navyBase : colors.steelGray,
                }}
              >
                {step > i + 1 ? "✓" : i + 1}
              </div>
              <span className="text-[10px] font-medium" style={{ color: step >= i + 1 ? colors.white : colors.steelGray }}>
                {label}
              </span>
              {i < STEP_LABELS.length - 1 && <span className="text-steel-gray">→</span>}
            </div>
          ))}
        </div>

        {/* Body */}
        <div className="flex-1 overflow-y-auto p-5">
          {error && (
            <div className="mb-4 rounded-md border p-3 text-xs" style={{ borderColor: `${colors.fail}40`, background: `${colors.fail}10`, color: colors.fail }}>
              {error}
            </div>
          )}

          {/* Step 1: Metadata */}
          {step === 1 && (
            <div className="grid grid-cols-2 gap-4">
              <div>
                <label className="mb-1 block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
                  Vessel name
                </label>
                <input type="text" value={vessel} onChange={(e) => setVessel(e.target.value)}
                  placeholder="e.g., RV Southern Surveyor"
                  className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 text-sm text-white focus:outline-none" />
              </div>
              <div>
                <label className="mb-1 block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
                  Sonar / MBES system
                </label>
                <input type="text" value={sonar} onChange={(e) => setSonar(e.target.value)}
                  placeholder="e.g., Kongsberg EM 2040"
                  className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 text-sm text-white focus:outline-none" />
              </div>
              <div>
                <label className="mb-1 block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
                  Survey area
                </label>
                <input type="text" value={surveyArea} onChange={(e) => setSurveyArea(e.target.value)}
                  placeholder="e.g., Port of Rotterdam — Entrance Channel"
                  className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 text-sm text-white focus:outline-none" />
              </div>
              <div>
                <label className="mb-1 block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
                  Survey date
                </label>
                <input type="date" value={surveyDate} onChange={(e) => setSurveyDate(e.target.value)}
                  className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 text-sm text-white focus:outline-none" />
              </div>
              <div>
                <label className="mb-1 block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
                  Coordinate system (EPSG)
                </label>
                <input type="text" value={epsg} onChange={(e) => setEpsg(e.target.value)}
                  placeholder="e.g., 4326 (WGS84) or 28355 (MGA Zone 55)"
                  className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 font-mono text-sm text-white focus:outline-none" />
              </div>
              <div>
                <label className="mb-1 block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
                  Client name
                </label>
                <input type="text" value={clientName} onChange={(e) => setClientName(e.target.value)}
                  placeholder="e.g., Port of Rotterdam Authority"
                  className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 text-sm text-white focus:outline-none" />
              </div>
              <div className="col-span-2">
                <label className="mb-1 block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
                  Surveyor name (responsible party)
                </label>
                <input type="text" value={surveyorName} onChange={(e) => setSurveyorName(e.target.value)}
                  placeholder="e.g., Jane Doe, IHO Cat A Hydrographic Surveyor"
                  className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 text-sm text-white focus:outline-none" />
              </div>
              <p className="col-span-2 text-[10px] text-steel-gray">
                This metadata populates the ISO 19115 gmd:MD_Metadata XML bundled in the ZIP.
              </p>
            </div>
          )}

          {/* Step 2: Source files */}
          {step === 2 && (
            <div className="space-y-3">
              <p className="text-xs text-steel-gray">
                Add the files to bundle. Each file's description and type appears in the manifest.
                Missing files are skipped with a warning.
              </p>
              {sources.map((src, i) => (
                <div key={i} className="rounded-md border border-navy-border bg-navy-base p-3 space-y-2">
                  <div className="flex items-center gap-2">
                    <span className="rounded px-2 py-0.5 text-[10px] font-bold" style={{ background: "#6366F120", color: "#6366F1" }}>
                      FILE {i + 1}
                    </span>
                    {sources.length > 1 && (
                      <button onClick={() => removeSource(i)} className="ml-auto rounded p-1 text-steel-gray hover:text-fail">
                        <Trash2 className="h-3.5 w-3.5" />
                      </button>
                    )}
                  </div>
                  <div>
                    <label className="mb-0.5 block text-[10px] font-semibold uppercase tracking-wider text-steel-light">
                      Description
                    </label>
                    <input type="text" value={src.description} onChange={(e) => updateSource(i, "description", e.target.value)}
                      placeholder="e.g., Bathymetric surface grid (1m)"
                      className="w-full rounded-md border border-navy-border bg-navy-base px-2 py-1.5 text-xs text-white focus:outline-none" />
                  </div>
                  <div className="grid grid-cols-2 gap-2">
                    <div>
                      <label className="mb-0.5 block text-[10px] font-semibold uppercase tracking-wider text-steel-light">
                        File type
                      </label>
                      <select
                        value={src.fileType}
                        onChange={(e) => updateSource(i, "fileType", e.target.value as DeliverableFileType)}
                        className="w-full rounded-md border border-navy-border bg-navy-base px-2 py-1.5 text-xs text-white focus:outline-none"
                      >
                        {FILE_TYPES.map((t) => (
                          <option key={t.value} value={t.value}>{t.label}</option>
                        ))}
                      </select>
                    </div>
                    <div>
                      <label className="mb-0.5 block text-[10px] font-semibold uppercase tracking-wider text-steel-light">
                        File path
                      </label>
                      <input type="text" value={src.path} onChange={(e) => updateSource(i, "path", e.target.value)}
                        placeholder="/path/to/file.tif"
                        className="w-full rounded-md border border-navy-border bg-navy-base px-2 py-1.5 font-mono text-xs text-white focus:outline-none" />
                    </div>
                  </div>
                  {files.length > 0 && (
                    <details className="text-[10px] text-steel-gray">
                      <summary className="cursor-pointer hover:text-white">Pick from loaded files</summary>
                      <div className="mt-1 space-y-0.5">
                        {files.map((f) => (
                          <button
                            key={f.id}
                            onClick={() => updateSource(i, "path", f.path)}
                            className="block w-full truncate rounded px-1 py-0.5 text-left font-mono hover:bg-navy-elevated"
                          >
                            {f.name} ({f.kind})
                          </button>
                        ))}
                      </div>
                    </details>
                  )}
                </div>
              ))}
              <button
                onClick={addSource}
                className="flex items-center gap-1.5 rounded-md border border-dashed border-navy-border px-3 py-2 text-xs text-steel-light hover:border-steel-light hover:text-white"
              >
                <Plus className="h-3.5 w-3.5" /> Add File
              </button>
            </div>
          )}

          {/* Step 3: Generate */}
          {step === 3 && (
            <div className="space-y-4">
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="mb-1 block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
                    Project / ZIP folder name
                  </label>
                  <input type="text" value={projectName} onChange={(e) => setProjectName(e.target.value)}
                    placeholder="e.g., Rotterdam_2026-06_Entrance"
                    className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 text-sm text-white focus:outline-none" />
                </div>
                <div>
                  <label className="mb-1 block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
                    Output ZIP path
                  </label>
                  <input type="text" value={outputPath} onChange={(e) => setOutputPath(e.target.value)}
                    className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 font-mono text-sm text-white focus:outline-none" />
                </div>
              </div>
              <div className="rounded-md border border-navy-border bg-navy-base p-3 text-xs">
                <div className="mb-2 font-semibold text-white">Summary</div>
                <div className="space-y-0.5 text-steel-light">
                  <div>Vessel: <span className="font-mono">{vessel || "—"}</span></div>
                  <div>Survey area: <span className="font-mono">{surveyArea || "—"}</span></div>
                  <div>Date: <span className="font-mono">{surveyDate}</span></div>
                  <div>Files to bundle: <span className="font-mono">{sources.filter((s) => s.path).length}</span></div>
                </div>
              </div>
              <button
                onClick={handleGenerate}
                disabled={generating}
                className="flex items-center gap-2 rounded-md px-5 py-2.5 text-sm font-bold transition-colors disabled:opacity-40"
                style={{ background: "#6366F1", color: "white" }}
              >
                {generating ? <Loader2 className="h-4 w-4 animate-spin" /> : <Package className="h-4 w-4" />}
                {generating ? "Generating package…" : "Generate Deliverable Package"}
              </button>
            </div>
          )}

          {/* Step 4: Review */}
          {step === 4 && result && (
            <div className="space-y-4">
              <div className="grid grid-cols-3 gap-2">
                <ResultTile label="Files Bundled" value={result.file_count.toString()} color="#6366F1" />
                <ResultTile label="Uncompressed" value={`${(result.total_size_bytes / 1024).toFixed(0)} KB`} color={colors.steelLight} />
                <ResultTile label="ZIP Size" value={`${(result.zip_size_bytes / 1024).toFixed(0)} KB`} color={colors.pass} />
              </div>

              {result.warnings.length > 0 && (
                <div
                  className="flex items-start gap-2 rounded-md border p-3 text-xs"
                  style={{ borderColor: "#F59E0B40", background: "#F59E0B10", color: "#F59E0B" }}
                >
                  <AlertTriangle className="mt-0.5 h-4 w-4 flex-shrink-0" />
                  <div>
                    <div className="font-semibold">{result.warnings.length} warnings</div>
                    <ul className="mt-1 list-disc pl-4 text-[10px]">
                      {result.warnings.slice(0, 5).map((w, i) => (
                        <li key={i}>{w}</li>
                      ))}
                    </ul>
                  </div>
                </div>
              )}

              <div>
                <h4 className="mb-2 text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
                  Bundled Files
                </h4>
                <div className="max-h-40 overflow-y-auto rounded-md border border-navy-border">
                  <table className="w-full text-left text-[10px]">
                    <thead className="sticky top-0 bg-navy-panel text-steel-gray">
                      <tr>
                        <th className="px-2 py-1.5">Description</th>
                        <th className="px-2 py-1.5">Type</th>
                        <th className="px-2 py-1.5 text-right">Size (KB)</th>
                        <th className="px-2 py-1.5">Hash (short)</th>
                        <th className="px-2 py-1.5">Status</th>
                      </tr>
                    </thead>
                    <tbody>
                      {result.files.map((f, i) => (
                        <tr key={i} className="border-t border-navy-border">
                          <td className="px-2 py-1 text-white">{f.description}</td>
                          <td className="px-2 py-1 text-steel-light">{f.file_type}</td>
                          <td className="px-2 py-1 text-right font-mono text-steel-light">
                            {f.size_bytes > 0 ? (f.size_bytes / 1024).toFixed(0) : "—"}
                          </td>
                          <td className="px-2 py-1 font-mono text-steel-light">{f.sha256_short}</td>
                          <td className="px-2 py-1" style={{ color: f.bundled ? colors.pass : colors.fail }}>
                            ● {f.bundled ? "OK" : "FAIL"}
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </div>

              <button
                onClick={() => setStep(5)}
                className="flex items-center gap-2 rounded-md px-5 py-2 text-sm font-bold transition-colors"
                style={{ background: colors.pass, color: colors.navyBase }}
              >
                <CheckCircle2 className="h-4 w-4" /> Complete
              </button>
            </div>
          )}

          {/* Step 5: Done */}
          {step === 5 && result && (
            <div className="flex flex-col items-center justify-center py-10">
              <CheckCircle2 className="mb-3 h-12 w-12" style={{ color: colors.pass }} />
              <h3 className="text-lg font-bold text-white">Deliverable Package Ready</h3>
              <p className="mt-1 text-sm text-steel-light">
                ZIP written to: <span className="font-mono">{outputPath}</span>
              </p>
              <p className="mt-2 text-xs text-steel-gray">
                {result.file_count} files bundled. Total: {(result.zip_size_bytes / 1024).toFixed(0)} KB.
              </p>
              <p className="mt-2 max-w-md text-center text-[10px] text-steel-gray">
                The ZIP includes a branded manifest.html (open in browser for an index of all files with hashes)
                and an ISO 19115 metadata.xml suitable for submission to port authorities and hydrographic offices.
              </p>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between border-t border-navy-border px-5 py-3">
          <button
            onClick={() => setStep((s) => Math.max(1, s - 1) as Step)}
            disabled={step === 1 || step === 5}
            className="flex items-center gap-1 text-xs text-steel-light hover:text-white disabled:opacity-30"
          >
            <ArrowLeft className="h-3 w-3" /> Back
          </button>
          {step < 3 && (
            <button
              onClick={() => setStep((s) => (s + 1) as Step)}
              disabled={!canNext}
              className="flex items-center gap-1 rounded-md px-4 py-1.5 text-xs font-medium disabled:opacity-40"
              style={{ background: canNext ? "#6366F1" : colors.steelGray, color: "white" }}
            >
              Next <ArrowRight className="h-3 w-3" />
            </button>
          )}
          {step === 5 && (
            <button
              onClick={onClose}
              className="flex items-center gap-1 rounded-md px-4 py-1.5 text-xs font-medium"
              style={{ background: colors.pass, color: colors.navyBase }}
            >
              <Download className="h-3 w-3" /> Finish
            </button>
          )}
        </div>
      </div>
    </div>
  );
}

function ResultTile({ label, value, color }: { label: string; value: string; color: string }) {
  return (
    <div className="rounded-md border p-2.5" style={{ borderColor: `${color}40`, background: `${color}10` }}>
      <div className="text-[9px] uppercase tracking-wider" style={{ color }}>{label}</div>
      <div className="mt-0.5 font-mono text-sm font-bold text-white">{value}</div>
    </div>
  );
}
