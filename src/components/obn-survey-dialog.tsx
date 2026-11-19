/**
 * OBN (Ocean Bottom Node) Survey Suite — 4DNav & Node Dashboard Automation.
 *
 * Automates the offshore hydrographic surveyor workflow on OBN vessels:
 *   1. Zero-friction ingest of 4DNav NavView logs and Node Dashboard CSVs.
 *   2. Automated geodetic tolerance auditing (Green <= 5m, Amber 5-10m, Red > 10m).
 *   3. 1-Click UKOOA SPS (R-cards) & IOGP P1/11 deliverables generation.
 *   4. 12-Hour Shift Handover and Daily Progress Report (DPR) compiler with SHA-256 seal.
 */

import { useState, useMemo, useEffect, useId } from "react";
import {
  Radio,
  Download,
  Copy,
  Check,
  AlertTriangle,
  FileText,
  Crosshair,
  ListFilter,
  Sparkles,
  Layers,
  ChevronRight,
  ShieldCheck,
  Clock,
} from "lucide-react";
import { colors } from "@/lib/tokens";
import { DialogShell, DialogButton } from "@/components/dialog-shell";
import {
  obnEvaluateQc,
  obnExportSps,
  obnExportP111,
  obnGenerateShiftDpr,
  obnGenerateSampleProject,
  type ObnQcSummary,
  type ObnHeader,
  type ObnShiftDpr,
} from "@/lib/tauri-ipc";

interface Props {
  open: boolean;
  onClose: () => void;
}

type TabKey = "qc_table" | "bullseye" | "exporters" | "dpr_handover" | "ingest";
type FilterKey = "all" | "flagged" | "in_spec";

export function ObnSurveyDialog({ open, onClose }: Props) {
  const [activeTab, setActiveTab] = useState<TabKey>("qc_table");
  const [filter, setFilter] = useState<FilterKey>("all");
  const [loading, setLoading] = useState(false);
  const [copied, setCopied] = useState(false);

  // Raw data inputs
  const [plannedContent, setPlannedContent] = useState("");
  const [aslaidContent, setAslaidContent] = useState("");
  const [toleranceSpecM, setToleranceSpecM] = useState<number>(5.0);

  // Header & Shift state
  const [header, setHeader] = useState<ObnHeader>({
    survey_name: "Triton Deep OBN Seismic Scope",
    client_name: "North Sea Petroleum Ltd",
    contractor_name: "UTM Offshore Survey Team",
    vessel_name: "MV Coastal Navigator",
    crs_name: "ED50 / UTM Zone 31N",
    epsg_code: 23031,
    date_str: "2026-10-01",
  });

  const [shift, setShift] = useState<ObnShiftDpr>({
    survey_name: "Triton Deep OBN Seismic Scope",
    client_name: "North Sea Petroleum Ltd",
    vessel_name: "MV Coastal Navigator",
    shift_date: "2026-10-01",
    shift_type: "Day Shift (00:00 - 12:00)",
    surveyor_name: "Senior Surveyor",
    total_planned_patch: 40,
    nodes_deployed_shift: 40,
    nodes_retrieved_shift: 0,
    deployment_rate_per_hour: 3.33,
    average_cycle_time_mins: 18.0,
    operational_hours: 10.5,
    weather_standby_hours: 1.0,
    technical_downtime_hours: 0.5,
    general_remarks: "Node deployment on Lines 01-04 completed. Station STN_0303 landed 11.7m off target due to high bottom currents; flagged for client rep review.",
    audit_hash: "",
  });

  // Exported deliverables
  const [qcSummary, setQcSummary] = useState<ObnQcSummary | null>(null);
  const [exportFormat, setExportFormat] = useState<"sps" | "p111">("sps");
  const [exportedText, setExportedText] = useState("");
  const [dprText, setDprText] = useState("");

  const surveyId = useId();
  const vesselId = useId();
  const shiftTypeId = useId();
  const surveyorId = useId();
  const opHoursId = useId();
  const weatherHoursId = useId();
  const techHoursId = useId();
  const plannedTextId = useId();
  const aslaidTextId = useId();

  // Load sample demo on first open if empty
  useEffect(() => {
    if (open && !qcSummary && !plannedContent) {
      handleLoadSampleData();
    }
  }, [open]);

  // Compute QC whenever data changes
  const runEvaluation = async (pContent: string, aContent: string, tol: number) => {
    if (!aContent.trim()) return;
    setLoading(true);
    try {
      const summary = await obnEvaluateQc({
        plannedContent: pContent,
        aslaidContent: aContent,
        toleranceSpecM: tol,
      });
      setQcSummary(summary);

      // Update shift stats automatically
      const depRate = summary.total_nodes > 0 ? summary.total_nodes / 12.0 : 0;
      const cycleMins = depRate > 0 ? 60.0 / depRate : 0;
      setShift(prev => ({
        ...prev,
        nodes_deployed_shift: summary.total_nodes,
        deployment_rate_per_hour: Number(depRate.toFixed(2)),
        average_cycle_time_mins: Number(cycleMins.toFixed(1)),
      }));
    } catch (err) {
      console.error("OBN QC evaluation failed:", err);
    } finally {
      setLoading(false);
    }
  };

  const handleLoadSampleData = async () => {
    setLoading(true);
    try {
      const sample = await obnGenerateSampleProject();
      setPlannedContent(sample.planned_csv);
      setAslaidContent(sample.aslaid_csv);
      setHeader(sample.default_header);
      await runEvaluation(sample.planned_csv, sample.aslaid_csv, toleranceSpecM);
    } catch (err) {
      console.error("Failed to load sample project:", err);
    } finally {
      setLoading(false);
    }
  };

  // Re-generate export texts when tab changes or data updates
  useEffect(() => {
    if (!qcSummary) return;

    if (activeTab === "exporters") {
      if (exportFormat === "sps") {
        obnExportSps(qcSummary.nodes, header).then(setExportedText);
      } else {
        obnExportP111(qcSummary.nodes, header).then(setExportedText);
      }
    } else if (activeTab === "dpr_handover") {
      obnGenerateShiftDpr(shift, qcSummary).then(setDprText);
    }
  }, [activeTab, exportFormat, qcSummary, header, shift]);

  // Filtered nodes
  const filteredNodes = useMemo(() => {
    if (!qcSummary) return [];
    if (filter === "flagged") {
      return qcSummary.nodes.filter(n => n.tolerance !== "in_spec");
    }
    if (filter === "in_spec") {
      return qcSummary.nodes.filter(n => n.tolerance === "in_spec");
    }
    return qcSummary.nodes;
  }, [qcSummary, filter]);

  const handleCopy = (text: string) => {
    navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const handleDownload = (filename: string, text: string) => {
    const blob = new Blob([text], { type: "text/plain;charset=utf-8" });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = filename;
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    URL.revokeObjectURL(url);
  };

  return (
    <DialogShell
      open={open}
      onClose={onClose}
      title="OBN Survey Suite — 4DNav & Node Dashboard"
      icon={<Radio className="h-5 w-5" />}
      iconColor={colors.marine}
      maxWidth="max-w-5xl"
      subtitle="Ocean Bottom Node deployment automation, spatial QC auditing, UKOOA SPS / IOGP P1/11 compiler & 12h shift DPR engine"
      footerHint={
        qcSummary
          ? `Active scope: ${qcSummary.total_nodes} nodes | ${qcSummary.compliance_percentage.toFixed(1)}% compliance (≤${toleranceSpecM}m) | Mean offset: ${qcSummary.mean_radial_offset_m.toFixed(2)}m`
          : "Load or import 4DNav / Node Dashboard data to begin"
      }
      actions={
        <div className="flex items-center gap-2">
          <DialogButton
            variant="secondary"
            onClick={handleLoadSampleData}
            disabled={loading}
          >
            <Sparkles className="h-4 w-4 mr-1 text-sky-400" />
            Load Offshore Demo Array
          </DialogButton>
          <DialogButton variant="secondary" onClick={onClose}>
            Close
          </DialogButton>
        </div>
      }
    >
      <div className="space-y-4">
        {/* Top KPI Metrics Bar */}
        {qcSummary && (
          <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-6 gap-3">
            <div className="rounded-lg bg-neutral-900/60 border border-neutral-800 p-3">
              <div className="text-xs text-neutral-400 font-medium">Total Nodes</div>
              <div className="text-xl font-bold text-neutral-100 mt-1">
                {qcSummary.total_nodes}
              </div>
              <div className="text-[11px] text-neutral-500 mt-0.5">Laid on seabed</div>
            </div>

            <div className="rounded-lg bg-emerald-950/30 border border-emerald-800/40 p-3">
              <div className="text-xs text-emerald-400 font-medium">In Spec (≤{toleranceSpecM}m)</div>
              <div className="text-xl font-bold text-emerald-300 mt-1">
                {qcSummary.in_spec_count}
              </div>
              <div className="text-[11px] text-emerald-500/80 mt-0.5">
                {((qcSummary.in_spec_count / qcSummary.total_nodes) * 100).toFixed(1)}% of array
              </div>
            </div>

            <div className="rounded-lg bg-amber-950/30 border border-amber-800/40 p-3">
              <div className="text-xs text-amber-400 font-medium">Warning (5-10m)</div>
              <div className="text-xl font-bold text-amber-300 mt-1">
                {qcSummary.warning_count}
              </div>
              <div className="text-[11px] text-amber-500/80 mt-0.5">Acceptable with note</div>
            </div>

            <div className="rounded-lg bg-red-950/30 border border-red-800/40 p-3">
              <div className="text-xs text-red-400 font-medium">Out of Spec (&gt;10m)</div>
              <div className="text-xl font-bold text-red-300 mt-1">
                {qcSummary.out_of_spec_count}
              </div>
              <div className="text-[11px] text-red-500/80 mt-0.5">Flagged for re-lay</div>
            </div>

            <div className="rounded-lg bg-neutral-900/60 border border-neutral-800 p-3">
              <div className="text-xs text-neutral-400 font-medium">Mean Radial Offset</div>
              <div className="text-xl font-bold text-neutral-100 mt-1">
                {qcSummary.mean_radial_offset_m.toFixed(2)} m
              </div>
              <div className="text-[11px] text-neutral-500 mt-0.5">
                σ = {qcSummary.std_dev_radial_m.toFixed(2)} m
              </div>
            </div>

            <div className="rounded-lg bg-neutral-900/60 border border-neutral-800 p-3">
              <div className="text-xs text-neutral-400 font-medium">Deployment Pace</div>
              <div className="text-xl font-bold text-sky-400 mt-1">
                {shift.deployment_rate_per_hour.toFixed(1)} /hr
              </div>
              <div className="text-[11px] text-neutral-500 mt-0.5">
                ~{shift.average_cycle_time_mins.toFixed(0)} min cycle
              </div>
            </div>
          </div>
        )}

        {/* Tab Navigation */}
        <div className="flex border-b border-neutral-800">
          <button
            onClick={() => setActiveTab("qc_table")}
            className={`px-4 py-2 text-sm font-medium border-b-2 flex items-center gap-2 transition-colors ${
              activeTab === "qc_table"
                ? "border-sky-500 text-sky-400 bg-sky-500/10"
                : "border-transparent text-neutral-400 hover:text-neutral-200"
            }`}
          >
            <ListFilter className="h-4 w-4" />
            Live QC Table
          </button>

          <button
            onClick={() => setActiveTab("bullseye")}
            className={`px-4 py-2 text-sm font-medium border-b-2 flex items-center gap-2 transition-colors ${
              activeTab === "bullseye"
                ? "border-sky-500 text-sky-400 bg-sky-500/10"
                : "border-transparent text-neutral-400 hover:text-neutral-200"
            }`}
          >
            <Crosshair className="h-4 w-4" />
            Radial Bullseye Target
          </button>

          <button
            onClick={() => setActiveTab("exporters")}
            className={`px-4 py-2 text-sm font-medium border-b-2 flex items-center gap-2 transition-colors ${
              activeTab === "exporters"
                ? "border-sky-500 text-sky-400 bg-sky-500/10"
                : "border-transparent text-neutral-400 hover:text-neutral-200"
            }`}
          >
            <Download className="h-4 w-4" />
            SPS & P1/11 Deliverables
          </button>

          <button
            onClick={() => setActiveTab("dpr_handover")}
            className={`px-4 py-2 text-sm font-medium border-b-2 flex items-center gap-2 transition-colors ${
              activeTab === "dpr_handover"
                ? "border-sky-500 text-sky-400 bg-sky-500/10"
                : "border-transparent text-neutral-400 hover:text-neutral-200"
            }`}
          >
            <Clock className="h-4 w-4" />
            12h Shift Handover & DPR
          </button>

          <button
            onClick={() => setActiveTab("ingest")}
            className={`px-4 py-2 text-sm font-medium border-b-2 flex items-center gap-2 transition-colors ${
              activeTab === "ingest"
                ? "border-sky-500 text-sky-400 bg-sky-500/10"
                : "border-transparent text-neutral-400 hover:text-neutral-200"
            }`}
          >
            <Layers className="h-4 w-4" />
            Data Ingest (4DNav / Node Dashboard)
          </button>
        </div>

        {/* Tab 1: Live QC Table */}
        {activeTab === "qc_table" && (
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <span className="text-xs text-neutral-400 font-medium">Filter Nodes:</span>
                <button
                  onClick={() => setFilter("all")}
                  className={`px-2.5 py-1 text-xs rounded font-medium ${
                    filter === "all"
                      ? "bg-neutral-700 text-white"
                      : "bg-neutral-800/80 text-neutral-400 hover:text-neutral-200"
                  }`}
                >
                  All ({qcSummary?.total_nodes ?? 0})
                </button>
                <button
                  onClick={() => setFilter("flagged")}
                  className={`px-2.5 py-1 text-xs rounded font-medium flex items-center gap-1 ${
                    filter === "flagged"
                      ? "bg-red-900/60 text-red-200 border border-red-700/50"
                      : "bg-neutral-800/80 text-neutral-400 hover:text-red-300"
                  }`}
                >
                  <AlertTriangle className="h-3 w-3 text-red-400" />
                  Flagged Out-of-Spec ({((qcSummary?.warning_count ?? 0) + (qcSummary?.out_of_spec_count ?? 0))})
                </button>
                <button
                  onClick={() => setFilter("in_spec")}
                  className={`px-2.5 py-1 text-xs rounded font-medium ${
                    filter === "in_spec"
                      ? "bg-emerald-900/60 text-emerald-200 border border-emerald-700/50"
                      : "bg-neutral-800/80 text-neutral-400 hover:text-emerald-300"
                  }`}
                >
                  In Spec ({qcSummary?.in_spec_count ?? 0})
                </button>
              </div>

              <div className="flex items-center gap-2">
                <span className="text-xs text-neutral-400">Spec Tolerance:</span>
                <input
                  type="number"
                  step="0.5"
                  value={toleranceSpecM}
                  onChange={e => {
                    const v = parseFloat(e.target.value) || 5.0;
                    setToleranceSpecM(v);
                    runEvaluation(plannedContent, aslaidContent, v);
                  }}
                  className="w-16 px-2 py-0.5 text-xs bg-neutral-900 border border-neutral-700 rounded text-neutral-200 focus:outline-none focus:border-sky-500"
                />
                <span className="text-xs text-neutral-500">meters</span>
              </div>
            </div>

            <div className="border border-neutral-800 rounded-lg overflow-hidden max-h-[460px] overflow-y-auto">
              <table className="w-full text-left text-xs border-collapse">
                <thead className="bg-neutral-900/90 text-neutral-400 sticky top-0 border-b border-neutral-800 z-10">
                  <tr>
                    <th className="p-2.5 font-medium">Station ID</th>
                    <th className="p-2.5 font-medium">Node SN</th>
                    <th className="p-2.5 font-medium">Line</th>
                    <th className="p-2.5 font-medium">Planned E / N</th>
                    <th className="p-2.5 font-medium">As-Laid Fix</th>
                    <th className="p-2.5 font-medium text-right">ΔE (m)</th>
                    <th className="p-2.5 font-medium text-right">ΔN (m)</th>
                    <th className="p-2.5 font-medium text-right">Radial Offset</th>
                    <th className="p-2.5 font-medium text-right">USBL 1σ</th>
                    <th className="p-2.5 font-medium text-center">Status</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-neutral-800/60 font-mono">
                  {filteredNodes.map(node => (
                    <tr
                      key={node.station_id}
                      className={`hover:bg-neutral-800/30 ${
                        node.tolerance === "out_of_spec"
                          ? "bg-red-950/20"
                          : node.tolerance === "warning"
                          ? "bg-amber-950/15"
                          : ""
                      }`}
                    >
                      <td className="p-2 font-semibold text-neutral-200">{node.station_id}</td>
                      <td className="p-2 text-neutral-400">{node.serial_number}</td>
                      <td className="p-2 text-neutral-400 font-sans">{node.line_name}</td>
                      <td className="p-2 text-neutral-400">
                        {node.planned_easting.toFixed(1)}, {node.planned_northing.toFixed(1)}
                      </td>
                      <td className="p-2 text-neutral-300">
                        {node.aslaid_easting.toFixed(1)}, {node.aslaid_northing.toFixed(1)}
                      </td>
                      <td
                        className={`p-2 text-right ${
                          node.delta_easting >= 0 ? "text-neutral-300" : "text-neutral-400"
                        }`}
                      >
                        {node.delta_easting >= 0 ? "+" : ""}
                        {node.delta_easting.toFixed(2)}
                      </td>
                      <td
                        className={`p-2 text-right ${
                          node.delta_northing >= 0 ? "text-neutral-300" : "text-neutral-400"
                        }`}
                      >
                        {node.delta_northing >= 0 ? "+" : ""}
                        {node.delta_northing.toFixed(2)}
                      </td>
                      <td className="p-2 text-right font-bold">
                        <span
                          className={`inline-flex items-center px-2 py-0.5 rounded text-xs ${
                            node.tolerance === "in_spec"
                              ? "text-emerald-400 bg-emerald-950/60 border border-emerald-800/40"
                              : node.tolerance === "warning"
                              ? "text-amber-400 bg-amber-950/60 border border-amber-800/40"
                              : "text-red-400 bg-red-950/60 border border-red-800/50"
                          }`}
                        >
                          {node.radial_offset_m.toFixed(2)} m
                        </span>
                      </td>
                      <td className="p-2 text-right text-neutral-400">
                        ±{node.usbl_uncertainty_m.toFixed(2)}m
                      </td>
                      <td className="p-2 text-center font-sans">
                        <span className="text-[11px] uppercase tracking-wider text-neutral-400">
                          {node.status}
                        </span>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        )}

        {/* Tab 2: Radial Bullseye Target */}
        {activeTab === "bullseye" && (
          <div className="space-y-4">
            <div className="flex items-center justify-between text-xs text-neutral-400">
              <span>Scatter of seabed node positions relative to target center (0, 0)</span>
              <div className="flex items-center gap-4">
                <span className="flex items-center gap-1">
                  <span className="w-2.5 h-2.5 rounded-full bg-emerald-500 inline-block"></span>
                  In-Spec (≤{toleranceSpecM}m)
                </span>
                <span className="flex items-center gap-1">
                  <span className="w-2.5 h-2.5 rounded-full bg-amber-500 inline-block"></span>
                  Warning (5-10m)
                </span>
                <span className="flex items-center gap-1">
                  <span className="w-2.5 h-2.5 rounded-full bg-red-500 inline-block"></span>
                  Out-of-Spec (&gt;10m)
                </span>
              </div>
            </div>

            {/* Radar / Bullseye Canvas */}
            <div className="flex justify-center items-center py-4 bg-neutral-950 rounded-xl border border-neutral-800 relative">
              <svg viewBox="-150 -150 300 300" className="w-[380px] h-[380px]">
                {/* Outer Grid Ring: 15m */}
                <circle cx="0" cy="0" r="135" fill="none" stroke="#262626" strokeWidth="1" strokeDasharray="3 3" />
                <text x="138" y="4" fill="#737373" fontSize="8" fontFamily="sans-serif">15m</text>

                {/* Warning Ring: 10m */}
                <circle cx="0" cy="0" r="90" fill="none" stroke="#f59e0b" strokeWidth="1.2" strokeDasharray="4 3" opacity="0.6" />
                <text x="93" y="4" fill="#f59e0b" fontSize="8" fontFamily="sans-serif">10m</text>

                {/* Client Spec Ring: 5m */}
                <circle cx="0" cy="0" r="45" fill="none" stroke="#10b981" strokeWidth="1.5" strokeDasharray="4 3" opacity="0.8" />
                <text x="48" y="4" fill="#10b981" fontSize="8" fontFamily="sans-serif">5m</text>

                {/* Crosshairs */}
                <line x1="-140" y1="0" x2="140" y2="0" stroke="#333333" strokeWidth="1" />
                <line x1="0" y1="-140" x2="0" y2="140" stroke="#333333" strokeWidth="1" />
                <circle cx="0" cy="0" r="2" fill="#38bdf8" />

                {/* Node Points: Scale 1m = 9px */}
                {qcSummary?.nodes.map(n => {
                  const cx = n.delta_easting * 9;
                  const cy = -n.delta_northing * 9; // invert Y for standard north-up
                  const color =
                    n.tolerance === "in_spec"
                      ? "#10b981"
                      : n.tolerance === "warning"
                      ? "#f59e0b"
                      : "#ef4444";

                  return (
                    <g key={n.station_id} className="cursor-pointer group">
                      <circle
                        cx={cx}
                        cy={cy}
                        r={n.tolerance === "out_of_spec" ? 4.5 : 3.5}
                        fill={color}
                        stroke="#000"
                        strokeWidth="0.8"
                        opacity="0.9"
                      />
                      <title>{`${n.station_id} (${n.serial_number})\nRadial: ${n.radial_offset_m.toFixed(2)}m\nΔE: ${n.delta_easting.toFixed(2)}m, ΔN: ${n.delta_northing.toFixed(2)}m`}</title>
                    </g>
                  );
                })}
              </svg>
            </div>
          </div>
        )}

        {/* Tab 3: SPS & P1/11 Deliverables */}
        {activeTab === "exporters" && (
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <span className="text-xs text-neutral-400 font-medium">Standard Format:</span>
                <button
                  onClick={() => setExportFormat("sps")}
                  className={`px-3 py-1 text-xs rounded font-medium ${
                    exportFormat === "sps"
                      ? "bg-sky-600 text-white"
                      : "bg-neutral-800 text-neutral-400 hover:text-neutral-200"
                  }`}
                >
                  UKOOA SPS Rev 2.1 (Receiver R-Cards)
                </button>
                <button
                  onClick={() => setExportFormat("p111")}
                  className={`px-3 py-1 text-xs rounded font-medium ${
                    exportFormat === "p111"
                      ? "bg-sky-600 text-white"
                      : "bg-neutral-800 text-neutral-400 hover:text-neutral-200"
                  }`}
                >
                  IOGP P1/11 (Geophysical Position Data)
                </button>
              </div>

              <div className="flex items-center gap-2">
                <button
                  onClick={() => handleCopy(exportedText)}
                  className="px-3 py-1 text-xs rounded bg-neutral-800 hover:bg-neutral-700 text-neutral-200 flex items-center gap-1"
                >
                  {copied ? <Check className="h-3.5 w-3.5 text-emerald-400" /> : <Copy className="h-3.5 w-3.5" />}
                  Copy File
                </button>
                <button
                  onClick={() =>
                    handleDownload(
                      exportFormat === "sps" ? "survey_nodes.rps" : "survey_nodes.p111",
                      exportedText
                    )
                  }
                  className="px-3 py-1 text-xs rounded bg-sky-600 hover:bg-sky-500 text-white flex items-center gap-1"
                >
                  <Download className="h-3.5 w-3.5" />
                  Download Deliverable
                </button>
              </div>
            </div>

            <div className="relative">
              <textarea
                readOnly
                value={exportedText}
                rows={16}
                className="w-full p-3 font-mono text-xs bg-neutral-950 border border-neutral-800 rounded-lg text-neutral-300 focus:outline-none selection:bg-sky-900 resize-none"
              />
              <div className="absolute bottom-3 right-3 text-[11px] text-neutral-500 bg-neutral-900/90 px-2 py-0.5 rounded border border-neutral-800">
                {qcSummary?.total_nodes} station records · Verified format
              </div>
            </div>
          </div>
        )}

        {/* Tab 4: 12h Shift Handover & DPR */}
        {activeTab === "dpr_handover" && (
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <div className="text-xs text-neutral-400">
                Certified Daily Progress Report & Shift Handover Document
              </div>
              <div className="flex items-center gap-2">
                <button
                  onClick={() => handleCopy(dprText)}
                  className="px-3 py-1 text-xs rounded bg-neutral-800 hover:bg-neutral-700 text-neutral-200 flex items-center gap-1"
                >
                  {copied ? <Check className="h-3.5 w-3.5 text-emerald-400" /> : <Copy className="h-3.5 w-3.5" />}
                  Copy DPR Markdown
                </button>
                <button
                  onClick={() => handleDownload("OBN_Shift_Handover_DPR.md", dprText)}
                  className="px-3 py-1 text-xs rounded bg-sky-600 hover:bg-sky-500 text-white flex items-center gap-1"
                >
                  <FileText className="h-3.5 w-3.5" />
                  Download Handover Sheet
                </button>
              </div>
            </div>

            {/* Shift Form Inputs */}
            <div className="grid grid-cols-2 sm:grid-cols-4 gap-2 bg-neutral-900/50 p-3 rounded-lg border border-neutral-800 text-xs">
              <div>
                <label htmlFor={surveyId} className="text-neutral-400 block mb-1">Survey Name</label>
                <input
                  id={surveyId}
                  type="text"
                  value={shift.survey_name}
                  onChange={e => setShift({ ...shift, survey_name: e.target.value })}
                  className="w-full px-2 py-1 bg-neutral-950 border border-neutral-700 rounded text-neutral-200"
                />
              </div>
              <div>
                <label htmlFor={vesselId} className="text-neutral-400 block mb-1">Vessel</label>
                <input
                  id={vesselId}
                  type="text"
                  value={shift.vessel_name}
                  onChange={e => setShift({ ...shift, vessel_name: e.target.value })}
                  className="w-full px-2 py-1 bg-neutral-950 border border-neutral-700 rounded text-neutral-200"
                />
              </div>
              <div>
                <label htmlFor={shiftTypeId} className="text-neutral-400 block mb-1">Shift Type</label>
                <input
                  id={shiftTypeId}
                  type="text"
                  value={shift.shift_type}
                  onChange={e => setShift({ ...shift, shift_type: e.target.value })}
                  className="w-full px-2 py-1 bg-neutral-950 border border-neutral-700 rounded text-neutral-200"
                />
              </div>
              <div>
                <label htmlFor={surveyorId} className="text-neutral-400 block mb-1">Surveyor on Duty</label>
                <input
                  id={surveyorId}
                  type="text"
                  value={shift.surveyor_name}
                  onChange={e => setShift({ ...shift, surveyor_name: e.target.value })}
                  className="w-full px-2 py-1 bg-neutral-950 border border-neutral-700 rounded text-neutral-200"
                />
              </div>
              <div>
                <label htmlFor={opHoursId} className="text-neutral-400 block mb-1">Operational Hours</label>
                <input
                  id={opHoursId}
                  type="number"
                  step="0.5"
                  value={shift.operational_hours}
                  onChange={e => setShift({ ...shift, operational_hours: parseFloat(e.target.value) || 0 })}
                  className="w-full px-2 py-1 bg-neutral-950 border border-neutral-700 rounded text-neutral-200"
                />
              </div>
              <div>
                <label htmlFor={weatherHoursId} className="text-neutral-400 block mb-1">Weather Standby (hrs)</label>
                <input
                  id={weatherHoursId}
                  type="number"
                  step="0.5"
                  value={shift.weather_standby_hours}
                  onChange={e => setShift({ ...shift, weather_standby_hours: parseFloat(e.target.value) || 0 })}
                  className="w-full px-2 py-1 bg-neutral-950 border border-neutral-700 rounded text-neutral-200"
                />
              </div>
              <div>
                <label htmlFor={techHoursId} className="text-neutral-400 block mb-1">Tech Downtime (hrs)</label>
                <input
                  id={techHoursId}
                  type="number"
                  step="0.5"
                  value={shift.technical_downtime_hours}
                  onChange={e => setShift({ ...shift, technical_downtime_hours: parseFloat(e.target.value) || 0 })}
                  className="w-full px-2 py-1 bg-neutral-950 border border-neutral-700 rounded text-neutral-200"
                />
              </div>
              <div className="flex items-end">
                <button
                  onClick={() => {
                    if (qcSummary) {
                      obnGenerateShiftDpr(shift, qcSummary).then(setDprText);
                    }
                  }}
                  className="w-full py-1 bg-sky-600 hover:bg-sky-500 text-white rounded font-medium flex items-center justify-center gap-1"
                >
                  <ShieldCheck className="h-3.5 w-3.5" />
                  Re-Compile DPR
                </button>
              </div>
            </div>

            <textarea
              readOnly
              value={dprText}
              rows={14}
              className="w-full p-3 font-mono text-xs bg-neutral-950 border border-neutral-800 rounded-lg text-neutral-300 focus:outline-none selection:bg-sky-900 resize-none"
            />
          </div>
        )}

        {/* Tab 5: Data Ingest */}
        {activeTab === "ingest" && (
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div className="space-y-2">
              <label htmlFor={plannedTextId} className="text-xs font-medium text-neutral-300 flex items-center gap-1">
                <ChevronRight className="h-3.5 w-3.5 text-sky-400" />
                Planned Pre-Plot (CSV / SPS / P1/11)
              </label>
              <textarea
                id={plannedTextId}
                value={plannedContent}
                onChange={e => {
                  setPlannedContent(e.target.value);
                  runEvaluation(e.target.value, aslaidContent, toleranceSpecM);
                }}
                rows={12}
                placeholder="Station_ID,Line_Name,Easting,Northing,Depth&#10;STN_01,LINE_01,500000.0,6200000.0,1250.0"
                className="w-full p-2.5 font-mono text-xs bg-neutral-950 border border-neutral-800 rounded-lg text-neutral-200 focus:outline-none focus:border-sky-500 resize-none"
              />
            </div>

            <div className="space-y-2">
              <label htmlFor={aslaidTextId} className="text-xs font-medium text-neutral-300 flex items-center gap-1">
                <ChevronRight className="h-3.5 w-3.5 text-sky-400" />
                4DNav NavView Logs or Node Dashboard Export
              </label>
              <textarea
                id={aslaidTextId}
                value={aslaidContent}
                onChange={e => {
                  setAslaidContent(e.target.value);
                  runEvaluation(plannedContent, e.target.value, toleranceSpecM);
                }}
                rows={12}
                placeholder="NodeID,SerialNo,StationID,LineName,Easting,Northing,Depth,Timestamp,USBL_Sigma,WaterDepth,Status&#10;NODE_01,SN_7201,STN_01,LINE_01,500001.8,6200001.2,1250.2,2026-10-01T08:00:00Z,1.2,1250.2,Deployed"
                className="w-full p-2.5 font-mono text-xs bg-neutral-950 border border-neutral-800 rounded-lg text-neutral-200 focus:outline-none focus:border-sky-500 resize-none"
              />
            </div>
          </div>
        )}
      </div>
    </DialogShell>
  );
}
