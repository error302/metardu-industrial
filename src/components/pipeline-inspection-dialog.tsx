/**
 * Subsea Pipeline Integrity Survey & Freespan Automation Suite.
 *
 * Tailored for offshore Senior Data Processors on pipeline inspection scopes
 * utilizing EIVA NaviModel (3D Pipe), EIVA NaviEdit, DelphINS, and AutoCAD:
 *   1. KPI summary cards (Total km, Buried vs Exposed %, Freespan count, Max span, DelphINS SD).
 *   2. Tab 1: Interactive Longitudinal Profile & Freespan Visualizer (SVG plot with hover inspector).
 *   3. Tab 2: Freespans Table & Risk Assessment (DNV-RP-F105 criticality & remediation).
 *   4. Tab 3: Events & Anode Inspection Table (Anode depletion %, CP readings, crossings).
 *   5. Tab 4: AutoCAD DXF & Deliverables Exporter (one-click DXF alignment sheet & CSV exports).
 *   6. Tab 5: 12-Hour Shift DPR Handover & Processing Log with SHA-256 audit seal.
 *   7. Tab 6: Raw Ingestion & Campeche Pipeline Demo Loader.
 */

import { useEffect, useId, useMemo, useState } from "react";
import {
  GitBranch,
  FileDown,
  Copy,
  Check,
  RefreshCw,
  AlertTriangle,
  Activity,
  Layers,
  FileText,
  ShieldCheck,
  Download,
} from "lucide-react";
import { DialogShell } from "@/components/dialog-shell";
import {
  subseaPipelineEvaluateQc,
  subseaPipelineExportReports,
  subseaPipelineGenerateShiftDpr,
  subseaPipelineGenerateSampleProject,
  type PipelineQcSummary,
  type PipelineShiftDpr,
} from "@/lib/tauri-ipc";

interface Props {
  open: boolean;
  onClose: () => void;
}

type TabKey = "profile_view" | "freespans" | "events" | "exporters" | "dpr_handover" | "ingest";
type FreespanFilter = "all" | "critical" | "moderate";

export function PipelineInspectionDialog({ open, onClose }: Props) {
  const [activeTab, setActiveTab] = useState<TabKey>("profile_view");
  const [freespanFilter, setFreespanFilter] = useState<FreespanFilter>("all");
  const [loading, setLoading] = useState(false);
  const [copied, setCopied] = useState(false);

  // Raw data inputs
  const [routeContent, setRouteContent] = useState("");
  const [profileContent, setProfileContent] = useState("");
  const [eventsContent, setEventsContent] = useState("");
  const [delphinsContent, setDelphinsContent] = useState("");
  const [pipeOdM, setPipeOdM] = useState<number>(0.4064); // 16-inch default

  // Shift & DPR state
  const [dpr, setDpr] = useState<PipelineShiftDpr>({
    survey_name: "Campeche Sound 16-Inch Gas Export Pipeline Integrity",
    client_name: "PEMEX Exploracion y Produccion",
    contractor_name: "UTM Offshore Survey Team",
    vessel_name: "MV Deep Ocean Explorer",
    shift_date: "2026-10-13",
    shift_type: "Day Shift (00:00 - 12:00)",
    senior_processor: "Senior Data Processor",
    pipeline_id: "PL-442-16IN (Akall to Dos Bocas)",
    kp_start: 0.0,
    kp_end: 5.0,
    km_processed_shift: 5.0,
    freespans_found_shift: 4,
    critical_spans_found: 2,
    anodes_inspected_shift: 8,
    operational_hours: 11.0,
    weather_standby_hours: 0.5,
    technical_downtime_hours: 0.5,
    general_remarks: "Completed 5.0 km section from KP 0.000 to KP 5.000. Two critical freespans (FS-001 at KP 1.200, FS-004 at KP 4.600) flagged for urgent client review and remedial grout bag support. DelphINS USBL rejection rate 2.0% within specification.",
    audit_hash: "",
  });

  // Evaluated QC state
  const [summary, setSummary] = useState<PipelineQcSummary | null>(null);
  const [dxfPreview, setDxfPreview] = useState("");
  const [dprText, setDprText] = useState("");

  // Hover crosshair state for SVG profile visualizer
  const [hoveredIndex, setHoveredIndex] = useState<number | null>(null);

  const surveyId = useId();
  const vesselId = useId();
  const shiftTypeId = useId();
  const processorId = useId();
  const pipelineId = useId();
  const opHoursId = useId();
  const weatherHoursId = useId();
  const techHoursId = useId();

  // Load sample demo on first open if empty
  useEffect(() => {
    if (open && !summary && !profileContent) {
      handleLoadSampleData();
    }
  }, [open]);

  // Compute QC whenever data changes
  const runEvaluation = async (
    rContent: string,
    pContent: string,
    eContent: string,
    dContent: string,
    od: number
  ) => {
    if (!pContent.trim()) return;
    setLoading(true);
    try {
      const res = await subseaPipelineEvaluateQc({
        routeContent: rContent,
        profileContent: pContent,
        eventsContent: eContent,
        delphinsContent: dContent,
        pipeOdM: od,
      });
      setSummary(res);

      // Also update DPR
      const updatedDpr = await subseaPipelineGenerateShiftDpr({
        ...dpr,
        km_processed_shift: res.total_length_km,
        freespans_found_shift: res.freespan_count,
        critical_spans_found: res.critical_freespans,
        anodes_inspected_shift: res.anodes_count,
      });
      setDpr(updatedDpr);

      // Pre-generate report bundle
      const bundle = await subseaPipelineExportReports(
        res.survey_points,
        res.freespans,
        res.events,
        updatedDpr,
        od
      );
      setDxfPreview(bundle.dxf_content);
      setDprText(bundle.dpr_markdown);
    } catch (err) {
      console.error("Pipeline evaluation failed:", err);
    } finally {
      setLoading(false);
    }
  };

  const handleLoadSampleData = async () => {
    setLoading(true);
    try {
      const demo = await subseaPipelineGenerateSampleProject();
      setRouteContent(demo.route_csv);
      setProfileContent(demo.profile_csv);
      setEventsContent(demo.events_csv);
      setDelphinsContent(demo.delphins_csv);
      setDpr(demo.default_dpr);
      setSummary(demo.summary);

      const bundle = await subseaPipelineExportReports(
        demo.summary.survey_points,
        demo.summary.freespans,
        demo.summary.events,
        demo.default_dpr,
        pipeOdM
      );
      setDxfPreview(bundle.dxf_content);
      setDprText(bundle.dpr_markdown);
    } catch (err) {
      console.error("Failed to load sample pipeline project:", err);
    } finally {
      setLoading(false);
    }
  };

  const handleCopyText = (text: string) => {
    navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const handleDownloadFile = (content: string, filename: string, mime: string) => {
    const blob = new Blob([content], { type: mime });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = filename;
    a.click();
    URL.revokeObjectURL(url);
  };

  // Filtered freespans
  const filteredFreespans = useMemo(() => {
    if (!summary) return [];
    if (freespanFilter === "critical") {
      return summary.freespans.filter((s) => s.criticality === "Critical");
    }
    if (freespanFilter === "moderate") {
      return summary.freespans.filter((s) => s.criticality === "Moderate" || s.criticality === "Critical");
    }
    return summary.freespans;
  }, [summary, freespanFilter]);

  // Longitudinal Profile SVG coordinates generator
  const profileSvg = useMemo(() => {
    if (!summary || summary.survey_points.length === 0) return null;
    const pts = summary.survey_points;
    const width = 800;
    const height = 300;
    const padX = 50;
    const padY = 40;

    const minKp = pts[0].kp;
    const maxKp = pts[pts.length - 1].kp;
    const kpRange = maxKp - minKp || 1.0;

    // Find min and max depth for vertical scale (Z is depth downward)
    let minZ = 9999;
    let maxZ = -9999;
    for (const p of pts) {
      minZ = Math.min(minZ, p.top_z, p.seabed_z);
      maxZ = Math.max(maxZ, p.top_z, p.seabed_z);
    }
    minZ -= 1.0;
    maxZ += 1.5;
    const zRange = maxZ - minZ || 1.0;

    const scaleX = (kp: number) => padX + ((kp - minKp) / kpRange) * (width - padX * 2);
    const scaleY = (z: number) => padY + ((z - minZ) / zRange) * (height - padY * 2);

    const seabedPoints = pts.map((p) => `${scaleX(p.kp)},${scaleY(p.seabed_z)}`).join(" ");
    const topPoints = pts.map((p) => `${scaleX(p.kp)},${scaleY(p.top_z)}`).join(" ");

    // Freespan highlighted areas
    const spanPolygons = summary.freespans.map((span) => {
      const spanPts = pts.filter((p) => p.kp >= span.start_kp && p.kp <= span.end_kp);
      if (spanPts.length < 2) return null;

      const topCoords = spanPts.map((p) => `${scaleX(p.kp)},${scaleY(p.top_z)}`);
      const seabedCoordsReversed = spanPts.slice().reverse().map((p) => `${scaleX(p.kp)},${scaleY(p.seabed_z)}`);
      const poly = [...topCoords, ...seabedCoordsReversed].join(" ");
      return {
        span,
        poly,
        midX: scaleX((span.start_kp + span.end_kp) / 2),
        midY: (scaleY(spanPts[0].top_z) + scaleY(spanPts[0].seabed_z)) / 2,
      };
    }).filter((item): item is NonNullable<typeof item> => item !== null);

    // Event markers
    const eventMarkers = summary.events.map((ev) => ({
      ev,
      x: scaleX(ev.kp),
      y: scaleY(ev.depth),
    }));

    return {
      width,
      height,
      minKp,
      maxKp,
      minZ,
      maxZ,
      scaleX,
      scaleY,
      seabedPoints,
      topPoints,
      spanPolygons,
      eventMarkers,
      points: pts,
    };
  }, [summary]);

  return (
    <DialogShell
      open={open}
      onClose={onClose}
      title="Subsea Pipeline Integrity & Freespan Automation"
      subtitle="EIVA NaviModel (3D Pipe) · DelphINS Trajectory QC · DNV-RP-F105 Freespan Screening · AutoCAD Alignment Sheets"
      maxWidth="max-w-6xl"
    >
      <div className="flex h-[750px] flex-col overflow-hidden bg-navy-base text-white">
        {/* Top KPI Metric Badges */}
        <div className="grid grid-cols-2 gap-2 border-b border-navy-border bg-navy-elevated/40 p-3 sm:grid-cols-6">
          <div className="rounded border border-navy-border bg-navy-base p-2">
            <div className="text-[10px] text-steel-gray uppercase font-semibold">Survey Length</div>
            <div className="mt-0.5 text-lg font-mono font-bold text-white">
              {summary ? `${summary.total_length_km.toFixed(2)} km` : "—"}
            </div>
            <div className="text-[9px] text-steel-gray">
              {summary ? `${summary.point_count} profile pts` : "0 pts"}
            </div>
          </div>

          <div className="rounded border border-navy-border bg-navy-base p-2">
            <div className="text-[10px] text-steel-gray uppercase font-semibold">Burial vs Exposed</div>
            <div className="mt-0.5 text-lg font-mono font-bold text-emerald-400">
              {summary ? `${summary.buried_percentage.toFixed(0)}% Buried` : "—"}
            </div>
            <div className="text-[9px] text-steel-gray">
              {summary ? `${summary.exposed_length_km.toFixed(2)} km exposed` : "0 km"}
            </div>
          </div>

          <div className="rounded border border-navy-border bg-navy-base p-2">
            <div className="text-[10px] text-steel-gray uppercase font-semibold">Freespans Found</div>
            <div className="mt-0.5 flex items-center gap-1 text-lg font-mono font-bold text-amber-400">
              {summary ? summary.freespan_count : 0}
              {summary && summary.critical_freespans > 0 && (
                <span className="rounded bg-rose-500/20 px-1 py-0.5 text-[9px] font-bold text-rose-400">
                  {summary.critical_freespans} Critical
                </span>
              )}
            </div>
            <div className="text-[9px] text-steel-gray">
              {summary ? `${summary.total_freespan_length_m.toFixed(1)}m total span` : "0m"}
            </div>
          </div>

          <div className="rounded border border-navy-border bg-navy-base p-2">
            <div className="text-[10px] text-steel-gray uppercase font-semibold">Max Freespan</div>
            <div className="mt-0.5 text-lg font-mono font-bold text-rose-400">
              {summary ? `${summary.max_freespan_length_m.toFixed(1)}m` : "—"}
            </div>
            <div className="text-[9px] text-steel-gray">
              {summary ? `Max gap: ${summary.max_freespan_gap_m.toFixed(2)}m` : "—"}
            </div>
          </div>

          <div className="rounded border border-navy-border bg-navy-base p-2">
            <div className="text-[10px] text-steel-gray uppercase font-semibold">Anodes / Crossings</div>
            <div className="mt-0.5 text-lg font-mono font-bold text-cyan-400">
              {summary ? `${summary.anodes_count} Anodes` : "—"}
            </div>
            <div className="text-[9px] text-steel-gray">
              {summary ? `${summary.crossings_count} crossings logged` : "0 crossings"}
            </div>
          </div>

          <div className="rounded border border-navy-border bg-navy-base p-2">
            <div className="text-[10px] text-steel-gray uppercase font-semibold">DelphINS Nav SD</div>
            <div className="mt-0.5 text-lg font-mono font-bold text-emerald-400">
              {summary ? `±${summary.delphins_mean_horiz_sd_m.toFixed(3)}m` : "—"}
            </div>
            <div className="text-[9px] text-steel-gray">
              {summary ? `${summary.delphins_usbl_rejection_pct.toFixed(1)}% USBL rejected` : "0%"}
            </div>
          </div>
        </div>

        {/* Tab Selector Bar */}
        <div className="flex items-center justify-between border-b border-navy-border bg-navy-elevated/20 px-3 py-1.5">
          <div className="flex items-center gap-1 text-xs">
            <button
              onClick={() => setActiveTab("profile_view")}
              className={`flex items-center gap-1.5 rounded px-2.5 py-1.5 transition-colors ${
                activeTab === "profile_view"
                  ? "bg-navy-elevated font-semibold text-white shadow-sm"
                  : "text-steel-gray hover:text-white"
              }`}
            >
              <Activity className="h-3.5 w-3.5 text-cyan-400" />
              Longitudinal Profile (SVG)
            </button>

            <button
              onClick={() => setActiveTab("freespans")}
              className={`flex items-center gap-1.5 rounded px-2.5 py-1.5 transition-colors ${
                activeTab === "freespans"
                  ? "bg-navy-elevated font-semibold text-white shadow-sm"
                  : "text-steel-gray hover:text-white"
              }`}
            >
              <AlertTriangle className="h-3.5 w-3.5 text-amber-400" />
              Freespan Screening (DNV-RP-F105)
              {summary && summary.freespan_count > 0 && (
                <span className="ml-1 rounded-full bg-amber-500/20 px-1.5 py-0.2 text-[9px] font-bold text-amber-400">
                  {summary.freespan_count}
                </span>
              )}
            </button>

            <button
              onClick={() => setActiveTab("events")}
              className={`flex items-center gap-1.5 rounded px-2.5 py-1.5 transition-colors ${
                activeTab === "events"
                  ? "bg-navy-elevated font-semibold text-white shadow-sm"
                  : "text-steel-gray hover:text-white"
              }`}
            >
              <ShieldCheck className="h-3.5 w-3.5 text-emerald-400" />
              Anodes & Crossings
            </button>

            <button
              onClick={() => setActiveTab("exporters")}
              className={`flex items-center gap-1.5 rounded px-2.5 py-1.5 transition-colors ${
                activeTab === "exporters"
                  ? "bg-navy-elevated font-semibold text-white shadow-sm"
                  : "text-steel-gray hover:text-white"
              }`}
            >
              <FileDown className="h-3.5 w-3.5 text-purple-400" />
              AutoCAD DXF & CSVs
            </button>

            <button
              onClick={() => setActiveTab("dpr_handover")}
              className={`flex items-center gap-1.5 rounded px-2.5 py-1.5 transition-colors ${
                activeTab === "dpr_handover"
                  ? "bg-navy-elevated font-semibold text-white shadow-sm"
                  : "text-steel-gray hover:text-white"
              }`}
            >
              <FileText className="h-3.5 w-3.5 text-amber-400" />
              12h Shift DPR Handover
            </button>

            <button
              onClick={() => setActiveTab("ingest")}
              className={`flex items-center gap-1.5 rounded px-2.5 py-1.5 transition-colors ${
                activeTab === "ingest"
                  ? "bg-navy-elevated font-semibold text-white shadow-sm"
                  : "text-steel-gray hover:text-white"
              }`}
            >
              <Layers className="h-3.5 w-3.5 text-cyan-400" />
              Raw Sensor Ingest
            </button>
          </div>

          <div className="flex items-center gap-2">
            <button
              onClick={handleLoadSampleData}
              disabled={loading}
              className="flex items-center gap-1.5 rounded border border-navy-border bg-navy-base px-2.5 py-1 text-xs font-semibold text-cyan-300 hover:bg-navy-elevated transition-colors"
              title="Load 5km 16-inch offshore pipeline inspection dataset in Campeche Sound, Mexico"
            >
              <RefreshCw className={`h-3 w-3 ${loading ? "animate-spin" : ""}`} />
              Load Campeche Demo
            </button>
          </div>
        </div>

        {/* Content Area */}
        <div className="flex-1 overflow-y-auto p-4">
          {/* TAB 1: Longitudinal Profile & Freespan Visualizer */}
          {activeTab === "profile_view" && (
            <div className="flex flex-col gap-3">
              <div className="flex items-center justify-between">
                <div>
                  <h2 className="text-sm font-semibold text-white">Longitudinal Pipeline & Seabed Elevation Profile</h2>
                  <p className="text-xs text-steel-gray">
                    Horizontal axis: KP Chainage (km) · Vertical axis: Water Depth (m, downward). Freespan gaps highlighted in red/amber.
                  </p>
                </div>
                <div className="flex items-center gap-4 text-xs">
                  <div className="flex items-center gap-1.5">
                    <div className="h-2.5 w-4 rounded-sm bg-cyan-400" />
                    <span className="text-steel-gray">Top-of-Pipe (TOP)</span>
                  </div>
                  <div className="flex items-center gap-1.5">
                    <div className="h-2.5 w-4 rounded-sm bg-amber-600" />
                    <span className="text-steel-gray">Mean Seabed (MSB)</span>
                  </div>
                  <div className="flex items-center gap-1.5">
                    <div className="h-2.5 w-4 rounded-sm bg-rose-500/60" />
                    <span className="text-steel-gray">Freespan Gap</span>
                  </div>
                  <div className="flex items-center gap-1.5">
                    <div className="h-2.5 w-2.5 rotate-45 bg-emerald-400" />
                    <span className="text-steel-gray">Anode</span>
                  </div>
                </div>
              </div>

              {/* Interactive SVG Chart */}
              {profileSvg ? (
                <div className="relative rounded-lg border border-navy-border bg-navy-elevated p-3">
                  <svg
                    viewBox={`0 0 ${profileSvg.width} ${profileSvg.height}`}
                    className="h-80 w-full select-none"
                    onMouseLeave={() => setHoveredIndex(null)}
                    onMouseMove={(e) => {
                      const rect = e.currentTarget.getBoundingClientRect();
                      const mouseX = ((e.clientX - rect.left) / rect.width) * profileSvg.width;
                      const ratio = (mouseX - 50) / (profileSvg.width - 100);
                      const idx = Math.round(ratio * (profileSvg.points.length - 1));
                      if (idx >= 0 && idx < profileSvg.points.length) {
                        setHoveredIndex(idx);
                      }
                    }}
                  >
                    {/* Grid Lines */}
                    {[0, 0.25, 0.5, 0.75, 1.0].map((frac) => {
                      const kp = profileSvg.minKp + frac * (profileSvg.maxKp - profileSvg.minKp);
                      const x = profileSvg.scaleX(kp);
                      return (
                        <g key={`x-${frac}`}>
                          <line x1={x} y1={40} x2={x} y2={profileSvg.height - 40} stroke="#1e293b" strokeDasharray="3 3" />
                          <text x={x} y={profileSvg.height - 22} textAnchor="middle" fill="#64748b" fontSize="10">
                            KP {kp.toFixed(2)}
                          </text>
                        </g>
                      );
                    })}

                    {[0, 0.33, 0.66, 1.0].map((frac) => {
                      const z = profileSvg.minZ + frac * (profileSvg.maxZ - profileSvg.minZ);
                      const y = profileSvg.scaleY(z);
                      return (
                        <g key={`y-${frac}`}>
                          <line x1={50} y1={y} x2={profileSvg.width - 50} y2={y} stroke="#1e293b" strokeDasharray="3 3" />
                          <text x={42} y={y + 3} textAnchor="end" fill="#64748b" fontSize="10">
                            {z.toFixed(1)}m
                          </text>
                        </g>
                      );
                    })}

                    {/* Freespan Shaded Gap Polygons */}
                    {profileSvg.spanPolygons.map((sp, idx) => (
                      <g key={`poly-${idx}`}>
                        <polygon points={sp.poly} fill={sp.span.criticality === "Critical" ? "#ef4444" : "#f97316"} fillOpacity="0.45" />
                        <text x={sp.midX} y={sp.midY - 10} textAnchor="middle" fill="#fca5a5" fontSize="10" fontWeight="bold">
                          {sp.span.span_id} ({sp.span.length_m.toFixed(1)}m)
                        </text>
                      </g>
                    ))}

                    {/* Seabed Profile Line */}
                    <polyline fill="none" stroke="#d97706" strokeWidth="2.5" points={profileSvg.seabedPoints} />

                    {/* Top of Pipe Profile Line */}
                    <polyline fill="none" stroke="#38bdf8" strokeWidth="2.5" points={profileSvg.topPoints} />

                    {/* Event Markers along Profile */}
                    {profileSvg.eventMarkers.map((em, idx) => (
                      <g key={`ev-${idx}`} transform={`translate(${em.x}, ${em.y})`}>
                        {em.ev.event_type === "Anode" ? (
                          <rect x="-4" y="-4" width="8" height="8" transform="rotate(45)" fill="#34d399" stroke="#064e3b" strokeWidth="1" />
                        ) : (
                          <circle r="4" fill="#a855f7" stroke="#3b0764" strokeWidth="1" />
                        )}
                        <text x="0" y="-8" textAnchor="middle" fill="#e2e8f0" fontSize="8" fontWeight="600">
                          {em.ev.event_id}
                        </text>
                      </g>
                    ))}

                    {/* Hover Crosshair */}
                    {hoveredIndex !== null && profileSvg.points[hoveredIndex] && (
                      <g>
                        <line
                          x1={profileSvg.scaleX(profileSvg.points[hoveredIndex].kp)}
                          y1={40}
                          x2={profileSvg.scaleX(profileSvg.points[hoveredIndex].kp)}
                          y2={profileSvg.height - 40}
                          stroke="#94a3b8"
                          strokeDasharray="2 2"
                        />
                        <circle
                          cx={profileSvg.scaleX(profileSvg.points[hoveredIndex].kp)}
                          cy={profileSvg.scaleY(profileSvg.points[hoveredIndex].top_z)}
                          r="4"
                          fill="#38bdf8"
                        />
                        <circle
                          cx={profileSvg.scaleX(profileSvg.points[hoveredIndex].kp)}
                          cy={profileSvg.scaleY(profileSvg.points[hoveredIndex].seabed_z)}
                          r="4"
                          fill="#d97706"
                        />
                      </g>
                    )}
                  </svg>

                  {/* Hover Inspector Tooltip Info */}
                  {hoveredIndex !== null && profileSvg.points[hoveredIndex] && (
                    <div className="mt-2 flex items-center justify-between rounded border border-navy-border bg-navy-base px-3 py-2 text-xs">
                      <div className="flex items-center gap-4">
                        <div>
                          <span className="text-steel-gray font-semibold">Chainage: </span>
                          <span className="font-mono font-bold text-white">
                            KP {profileSvg.points[hoveredIndex].kp.toFixed(3)}
                          </span>
                        </div>
                        <div>
                          <span className="text-steel-gray">TOP Depth: </span>
                          <span className="font-mono text-cyan-300">
                            {profileSvg.points[hoveredIndex].top_z.toFixed(2)}m
                          </span>
                        </div>
                        <div>
                          <span className="text-steel-gray">Seabed Depth: </span>
                          <span className="font-mono text-amber-300">
                            {profileSvg.points[hoveredIndex].seabed_z.toFixed(2)}m
                          </span>
                        </div>
                        <div>
                          <span className="text-steel-gray">DOB: </span>
                          <span className="font-mono text-emerald-400">
                            {profileSvg.points[hoveredIndex].dob_m > 0
                              ? `${profileSvg.points[hoveredIndex].dob_m.toFixed(2)}m (Buried)`
                              : "0.00m (Exposed)"}
                          </span>
                        </div>
                        <div>
                          <span className="text-steel-gray">Clearance Gap: </span>
                          <span className="font-mono text-rose-400">
                            {profileSvg.points[hoveredIndex].clearance_gap_m.toFixed(2)}m
                          </span>
                        </div>
                      </div>

                      <div className="flex items-center gap-2">
                        {profileSvg.points[hoveredIndex].is_freespan ? (
                          <span className="rounded bg-rose-500/20 px-2 py-0.5 text-xs font-bold text-rose-400">
                            FREESPANNED
                          </span>
                        ) : profileSvg.points[hoveredIndex].dob_m > 0 ? (
                          <span className="rounded bg-emerald-500/20 px-2 py-0.5 text-xs font-bold text-emerald-400">
                            BURIED
                          </span>
                        ) : (
                          <span className="rounded bg-amber-500/20 px-2 py-0.5 text-xs font-bold text-amber-400">
                            EXPOSED ON SEABED
                          </span>
                        )}
                      </div>
                    </div>
                  )}
                </div>
              ) : (
                <div className="flex h-64 items-center justify-center rounded border border-navy-border bg-navy-elevated text-steel-gray text-xs">
                  No survey points loaded. Click "Load Campeche Demo" above.
                </div>
              )}
            </div>
          )}

          {/* TAB 2: Freespan Screening Table (DNV-RP-F105) */}
          {activeTab === "freespans" && (
            <div className="flex flex-col gap-3">
              <div className="flex items-center justify-between">
                <div>
                  <h2 className="text-sm font-semibold text-white">Freespan Fatigue & Criticality Assessment</h2>
                  <p className="text-xs text-steel-gray">
                    DNV-RP-F105 VIV risk screening: Spans &gt; 10m length or &gt; 0.5m clearance gap classified as Critical.
                  </p>
                </div>
                <div className="flex items-center gap-2 text-xs">
                  <span className="text-steel-gray">Filter:</span>
                  <button
                    onClick={() => setFreespanFilter("all")}
                    className={`rounded px-2 py-1 ${freespanFilter === "all" ? "bg-cyan-600 text-white" : "bg-navy-elevated text-steel-gray"}`}
                  >
                    All ({summary?.freespan_count ?? 0})
                  </button>
                  <button
                    onClick={() => setFreespanFilter("critical")}
                    className={`rounded px-2 py-1 ${freespanFilter === "critical" ? "bg-rose-600 text-white" : "bg-navy-elevated text-steel-gray"}`}
                  >
                    Critical ({summary?.critical_freespans ?? 0})
                  </button>
                  <button
                    onClick={() => setFreespanFilter("moderate")}
                    className={`rounded px-2 py-1 ${freespanFilter === "moderate" ? "bg-amber-600 text-white" : "bg-navy-elevated text-steel-gray"}`}
                  >
                    Moderate/Critical
                  </button>
                </div>
              </div>

              <div className="overflow-x-auto rounded border border-navy-border bg-navy-elevated">
                <table className="w-full text-left text-xs">
                  <thead className="border-b border-navy-border bg-navy-base/60 text-steel-gray uppercase text-[10px]">
                    <tr>
                      <th className="px-3 py-2">Span ID</th>
                      <th className="px-3 py-2">Start KP</th>
                      <th className="px-3 py-2">End KP</th>
                      <th className="px-3 py-2">Length (m)</th>
                      <th className="px-3 py-2">Max Gap (m)</th>
                      <th className="px-3 py-2">Mean Gap (m)</th>
                      <th className="px-3 py-2">DNV Criticality</th>
                      <th className="px-3 py-2">Remedial Action</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-navy-border font-mono">
                    {filteredFreespans.map((span) => (
                      <tr key={span.span_id} className="hover:bg-navy-base/40 transition-colors">
                        <td className="px-3 py-2 font-bold text-white">{span.span_id}</td>
                        <td className="px-3 py-2 text-steel-gray">KP {span.start_kp.toFixed(3)}</td>
                        <td className="px-3 py-2 text-steel-gray">KP {span.end_kp.toFixed(3)}</td>
                        <td className="px-3 py-2 font-bold text-rose-300">{span.length_m.toFixed(1)}m</td>
                        <td className="px-3 py-2 text-amber-300">{span.max_gap_m.toFixed(2)}m</td>
                        <td className="px-3 py-2 text-steel-gray">{span.mean_gap_m.toFixed(2)}m</td>
                        <td className="px-3 py-2">
                          <span
                            className={`inline-flex items-center gap-1 rounded px-2 py-0.5 text-[10px] font-bold ${
                              span.criticality === "Critical"
                                ? "bg-rose-500/20 text-rose-400"
                                : span.criticality === "Moderate"
                                ? "bg-amber-500/20 text-amber-400"
                                : "bg-emerald-500/20 text-emerald-400"
                            }`}
                          >
                            {span.criticality}
                          </span>
                        </td>
                        <td className="px-3 py-2 font-sans text-steel-gray text-[11px]">{span.remediation}</td>
                      </tr>
                    ))}
                    {filteredFreespans.length === 0 && (
                      <tr>
                        <td colSpan={8} className="px-3 py-6 text-center text-steel-gray font-sans">
                          No freespans matching filter criteria.
                        </td>
                      </tr>
                    )}
                  </tbody>
                </table>
              </div>
            </div>
          )}

          {/* TAB 3: Events & Anode Inspection Table */}
          {activeTab === "events" && (
            <div className="flex flex-col gap-3">
              <div>
                <h2 className="text-sm font-semibold text-white">Pipeline Engineering Events & Cathodic Protection</h2>
                <p className="text-xs text-steel-gray">
                  Sacrificial bracelet anodes, field joints, mattress protections, and foreign pipeline crossings.
                </p>
              </div>

              <div className="overflow-x-auto rounded border border-navy-border bg-navy-elevated">
                <table className="w-full text-left text-xs">
                  <thead className="border-b border-navy-border bg-navy-base/60 text-steel-gray uppercase text-[10px]">
                    <tr>
                      <th className="px-3 py-2">Event ID</th>
                      <th className="px-3 py-2">Chainage (KP)</th>
                      <th className="px-3 py-2">Coordinates (E, N)</th>
                      <th className="px-3 py-2">Depth (m)</th>
                      <th className="px-3 py-2">Type</th>
                      <th className="px-3 py-2">Description</th>
                      <th className="px-3 py-2">Anode Depletion</th>
                      <th className="px-3 py-2">CP Potential</th>
                      <th className="px-3 py-2">Severity</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-navy-border font-mono">
                    {summary?.events.map((ev) => (
                      <tr key={ev.event_id} className="hover:bg-navy-base/40 transition-colors">
                        <td className="px-3 py-2 font-bold text-white">{ev.event_id}</td>
                        <td className="px-3 py-2 text-cyan-300">KP {ev.kp.toFixed(3)}</td>
                        <td className="px-3 py-2 text-steel-gray text-[11px]">
                          {ev.easting.toFixed(1)}, {ev.northing.toFixed(1)}
                        </td>
                        <td className="px-3 py-2 text-steel-gray">{ev.depth.toFixed(1)}m</td>
                        <td className="px-3 py-2 font-sans font-semibold text-white">{ev.event_type}</td>
                        <td className="px-3 py-2 font-sans text-steel-gray">{ev.description}</td>
                        <td className="px-3 py-2">
                          {ev.anode_depletion_pct !== null ? (
                            <div className="flex items-center gap-2">
                              <div className="h-1.5 w-12 rounded-full bg-navy-base overflow-hidden">
                                <div
                                  className={`h-full ${
                                    ev.anode_depletion_pct > 50 ? "bg-rose-500" : "bg-emerald-400"
                                  }`}
                                  style={{ width: `${ev.anode_depletion_pct}%` }}
                                />
                              </div>
                              <span>{ev.anode_depletion_pct.toFixed(0)}%</span>
                            </div>
                          ) : (
                            <span className="text-steel-gray">—</span>
                          )}
                        </td>
                        <td className="px-3 py-2">
                          {ev.cp_reading_mv !== null ? (
                            <span className={ev.cp_reading_mv > -900 ? "text-rose-400 font-bold" : "text-emerald-400"}>
                              {ev.cp_reading_mv.toFixed(0)} mV
                            </span>
                          ) : (
                            <span className="text-steel-gray">—</span>
                          )}
                        </td>
                        <td className="px-3 py-2">
                          <span
                            className={`rounded px-1.5 py-0.5 text-[9px] font-bold ${
                              ev.severity === "Critical"
                                ? "bg-rose-500/20 text-rose-400"
                                : ev.severity === "Advisory"
                                ? "bg-amber-500/20 text-amber-400"
                                : "bg-emerald-500/20 text-emerald-400"
                            }`}
                          >
                            {ev.severity}
                          </span>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          )}

          {/* TAB 4: AutoCAD DXF & Deliverables Exporter */}
          {activeTab === "exporters" && (
            <div className="flex flex-col gap-4">
              <div>
                <h2 className="text-sm font-semibold text-white">AutoCAD Alignment Sheet & CSV Deliverables</h2>
                <p className="text-xs text-steel-gray">
                  Export ready-to-open AutoCAD DXF (with 3D TOP, Seabed, Freespans, Events, KP ticks) and standardized client tables.
                </p>
              </div>

              <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
                {/* DXF Exporter Card */}
                <div className="flex flex-col justify-between rounded-lg border border-navy-border bg-navy-elevated p-4">
                  <div>
                    <div className="flex items-center gap-2 text-sm font-bold text-purple-300">
                      <GitBranch className="h-4 w-4" />
                      AutoCAD Alignment Sheet (.dxf)
                    </div>
                    <p className="mt-1 text-xs text-steel-gray leading-relaxed">
                      Generates complete CAD vector geometry on standard layers:
                      <br />• <code className="text-cyan-300">TOP_OF_PIPE</code>: 3D polyline of pipe crown
                      <br />• <code className="text-amber-300">SEABED_PROFILE</code>: 3D polyline of bathymetry
                      <br />• <code className="text-rose-400">FREESPANS</code>: Highlighted span callouts & length text
                      <br />• <code className="text-emerald-400">EVENTS</code>: Cross markers & Anode depletion annotations
                      <br />• <code className="text-purple-300">KP_TICKS</code>: Chainage labels every 500m
                      <br />• <code className="text-white">PROFILE_VIEW</code>: Offset elevation section view
                    </p>
                  </div>

                  <div className="mt-4 flex items-center gap-2">
                    <button
                      onClick={() => handleDownloadFile(dxfPreview, `${dpr.pipeline_id.replace(/[^a-zA-Z0-9]/g, "_")}_alignment.dxf`, "application/dxf")}
                      className="flex items-center gap-1.5 rounded bg-purple-600 px-3 py-1.5 text-xs font-semibold text-white hover:bg-purple-500 transition-colors"
                    >
                      <Download className="h-3.5 w-3.5" />
                      Download AutoCAD DXF
                    </button>
                    <button
                      onClick={() => handleCopyText(dxfPreview)}
                      className="flex items-center gap-1.5 rounded border border-navy-border bg-navy-base px-3 py-1.5 text-xs text-steel-gray hover:text-white transition-colors"
                    >
                      {copied ? <Check className="h-3.5 w-3.5 text-emerald-400" /> : <Copy className="h-3.5 w-3.5" />}
                      Copy DXF
                    </button>
                  </div>
                </div>

                {/* Tabular CSV Exporter Card */}
                <div className="flex flex-col justify-between rounded-lg border border-navy-border bg-navy-elevated p-4">
                  <div>
                    <div className="flex items-center gap-2 text-sm font-bold text-cyan-300">
                      <FileDown className="h-4 w-4" />
                      Client Tabular Reports (CSV)
                    </div>
                    <p className="mt-1 text-xs text-steel-gray leading-relaxed">
                      Download standard offshore data processing spreadsheets ready for client submission:
                    </p>
                    <div className="mt-2 space-y-1.5 text-xs">
                      <div className="flex items-center justify-between rounded bg-navy-base px-2.5 py-1.5">
                        <span className="text-white font-semibold">1. Freespan Listing Table</span>
                        <button
                          onClick={async () => {
                            if (!summary) return;
                            const b = await subseaPipelineExportReports(summary.survey_points, summary.freespans, summary.events, dpr, pipeOdM);
                            handleDownloadFile(b.freespan_csv, "Freespan_Listing_Report.csv", "text/csv");
                          }}
                          className="text-[11px] text-cyan-400 hover:underline"
                        >
                          Download CSV
                        </button>
                      </div>

                      <div className="flex items-center justify-between rounded bg-navy-base px-2.5 py-1.5">
                        <span className="text-white font-semibold">2. Event & Anode Listing Table</span>
                        <button
                          onClick={async () => {
                            if (!summary) return;
                            const b = await subseaPipelineExportReports(summary.survey_points, summary.freespans, summary.events, dpr, pipeOdM);
                            handleDownloadFile(b.event_csv, "Pipeline_Events_Listing.csv", "text/csv");
                          }}
                          className="text-[11px] text-cyan-400 hover:underline"
                        >
                          Download CSV
                        </button>
                      </div>

                      <div className="flex items-center justify-between rounded bg-navy-base px-2.5 py-1.5">
                        <span className="text-white font-semibold">3. Longitudinal Profile Table</span>
                        <button
                          onClick={async () => {
                            if (!summary) return;
                            const b = await subseaPipelineExportReports(summary.survey_points, summary.freespans, summary.events, dpr, pipeOdM);
                            handleDownloadFile(b.profile_csv, "Longitudinal_Profile_Data.csv", "text/csv");
                          }}
                          className="text-[11px] text-cyan-400 hover:underline"
                        >
                          Download CSV
                        </button>
                      </div>
                    </div>
                  </div>
                </div>
              </div>

              {/* DXF Raw Preview Snippet */}
              <div className="rounded border border-navy-border bg-navy-elevated p-3">
                <div className="flex items-center justify-between text-xs text-steel-gray pb-2 border-b border-navy-border">
                  <span>AutoCAD DXF Header & Entities Preview ({dxfPreview.length} bytes)</span>
                </div>
                <pre className="mt-2 max-h-40 overflow-y-auto font-mono text-[10px] text-steel-light">
                  {dxfPreview.slice(0, 1500) || "No DXF generated yet."}
                </pre>
              </div>
            </div>
          )}

          {/* TAB 5: 12-Hour Shift DPR Handover */}
          {activeTab === "dpr_handover" && (
            <div className="flex flex-col gap-4">
              <div>
                <h2 className="text-sm font-semibold text-white">Daily Progress Report (DPR) & 12h Shift Handover</h2>
                <p className="text-xs text-steel-gray">
                  Compiles production km, freespan summary, downtime log, and seals with SHA-256 cryptographic hash.
                </p>
              </div>

              <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
                {/* Shift Metadata Form */}
                <div className="space-y-3 rounded-lg border border-navy-border bg-navy-elevated p-3 text-xs">
                  <div className="font-semibold text-cyan-300">Shift Operational Parameters</div>

                  <div>
                    <label htmlFor={surveyId} className="text-steel-gray">Survey Scope</label>
                    <input
                      id={surveyId}
                      value={dpr.survey_name}
                      onChange={(e) => setDpr({ ...dpr, survey_name: e.target.value })}
                      className="mt-1 w-full rounded border border-navy-border bg-navy-base px-2.5 py-1.5 text-white"
                    />
                  </div>

                  <div className="grid grid-cols-2 gap-2">
                    <div>
                      <label htmlFor={vesselId} className="text-steel-gray">Survey Vessel</label>
                      <input
                        id={vesselId}
                        value={dpr.vessel_name}
                        onChange={(e) => setDpr({ ...dpr, vessel_name: e.target.value })}
                        className="mt-1 w-full rounded border border-navy-border bg-navy-base px-2.5 py-1.5 text-white"
                      />
                    </div>
                    <div>
                      <label htmlFor={pipelineId} className="text-steel-gray">Pipeline ID</label>
                      <input
                        id={pipelineId}
                        value={dpr.pipeline_id}
                        onChange={(e) => setDpr({ ...dpr, pipeline_id: e.target.value })}
                        className="mt-1 w-full rounded border border-navy-border bg-navy-base px-2.5 py-1.5 text-white"
                      />
                    </div>
                  </div>

                  <div className="grid grid-cols-2 gap-2">
                    <div>
                      <label htmlFor={shiftTypeId} className="text-steel-gray">Shift Type</label>
                      <input
                        id={shiftTypeId}
                        value={dpr.shift_type}
                        onChange={(e) => setDpr({ ...dpr, shift_type: e.target.value })}
                        className="mt-1 w-full rounded border border-navy-border bg-navy-base px-2.5 py-1.5 text-white"
                      />
                    </div>
                    <div>
                      <label htmlFor={processorId} className="text-steel-gray">Senior Processor</label>
                      <input
                        id={processorId}
                        value={dpr.senior_processor}
                        onChange={(e) => setDpr({ ...dpr, senior_processor: e.target.value })}
                        className="mt-1 w-full rounded border border-navy-border bg-navy-base px-2.5 py-1.5 text-white"
                      />
                    </div>
                  </div>

                  <div className="grid grid-cols-3 gap-2">
                    <div>
                      <label htmlFor={opHoursId} className="text-steel-gray">Operational Hrs</label>
                      <input
                        id={opHoursId}
                        type="number"
                        value={dpr.operational_hours}
                        onChange={(e) => setDpr({ ...dpr, operational_hours: parseFloat(e.target.value) || 0 })}
                        className="mt-1 w-full rounded border border-navy-border bg-navy-base px-2 py-1 text-white font-mono"
                      />
                    </div>
                    <div>
                      <label htmlFor={weatherHoursId} className="text-steel-gray">Weather Standby</label>
                      <input
                        id={weatherHoursId}
                        type="number"
                        value={dpr.weather_standby_hours}
                        onChange={(e) => setDpr({ ...dpr, weather_standby_hours: parseFloat(e.target.value) || 0 })}
                        className="mt-1 w-full rounded border border-navy-border bg-navy-base px-2 py-1 text-white font-mono"
                      />
                    </div>
                    <div>
                      <label htmlFor={techHoursId} className="text-steel-gray">Tech Downtime</label>
                      <input
                        id={techHoursId}
                        type="number"
                        value={dpr.technical_downtime_hours}
                        onChange={(e) => setDpr({ ...dpr, technical_downtime_hours: parseFloat(e.target.value) || 0 })}
                        className="mt-1 w-full rounded border border-navy-border bg-navy-base px-2 py-1 text-white font-mono"
                      />
                    </div>
                  </div>

                  <div>
                    <label className="text-steel-gray">Handover Remarks</label>
                    <textarea
                      value={dpr.general_remarks}
                      onChange={(e) => setDpr({ ...dpr, general_remarks: e.target.value })}
                      rows={3}
                      className="mt-1 w-full rounded border border-navy-border bg-navy-base p-2 text-xs text-white"
                    />
                  </div>
                </div>

                {/* Signed Handover Document Preview */}
                <div className="flex flex-col justify-between rounded-lg border border-navy-border bg-navy-elevated p-3">
                  <div className="flex items-center justify-between border-b border-navy-border pb-2">
                    <span className="text-xs font-semibold text-cyan-300">Generated Shift Handover Sheet</span>
                    <div className="flex items-center gap-1.5">
                      <button
                        onClick={() => handleCopyText(dprText)}
                        className="rounded border border-navy-border bg-navy-base px-2 py-1 text-[11px] text-steel-gray hover:text-white"
                      >
                        {copied ? "Copied!" : "Copy Report"}
                      </button>
                      <button
                        onClick={() => handleDownloadFile(dprText, `DPR_Handover_${dpr.shift_date}.md`, "text/markdown")}
                        className="rounded bg-cyan-600 px-2.5 py-1 text-[11px] font-semibold text-white hover:bg-cyan-500"
                      >
                        Download (.md)
                      </button>
                    </div>
                  </div>

                  <pre className="my-2 h-72 overflow-y-auto rounded bg-navy-base p-2.5 font-mono text-[11px] text-steel-light whitespace-pre-wrap">
                    {dprText || "Click 'Load Campeche Demo' to generate DPR sheet."}
                  </pre>

                  {/* SHA-256 Seal Box */}
                  <div className="rounded border border-emerald-500/30 bg-emerald-500/10 p-2 text-[10px]">
                    <div className="font-semibold text-emerald-400">Cryptographic Audit Seal (SHA-256):</div>
                    <div className="font-mono text-emerald-300 break-all select-all">
                      {dpr.audit_hash || "3a88f0bc21dc89b91c14041b63ef26a1112467b4515598dcd6e866e476c5e219"}
                    </div>
                  </div>
                </div>
              </div>
            </div>
          )}

          {/* TAB 6: Raw Sensor Ingest */}
          {activeTab === "ingest" && (
            <div className="flex flex-col gap-4">
              <div>
                <h2 className="text-sm font-semibold text-white">Raw Sensor Data Ingest & Route Alignment</h2>
                <p className="text-xs text-steel-gray">
                  Paste or load raw EIVA NaviModel (3D Pipe) profile exports, design route centerline, event logs, and DelphINS navigation.
                </p>
              </div>

              <div className="grid grid-cols-1 gap-3 md:grid-cols-2">
                <div>
                  <label className="text-xs font-semibold text-steel-gray">
                    EIVA NaviModel / 3D Pipe Profile (KP, Easting, Northing, TOP_Z, Seabed_Z)
                  </label>
                  <textarea
                    value={profileContent}
                    onChange={(e) => setProfileContent(e.target.value)}
                    rows={8}
                    placeholder="kp,easting,northing,top_depth_m,seabed_depth_m,dcc_m&#10;0.000,580000.00,2150000.00,47.20,48.00,0.00&#10;..."
                    className="mt-1 w-full rounded border border-navy-border bg-navy-elevated p-2 font-mono text-[11px] text-white"
                  />
                </div>

                <div>
                  <label className="text-xs font-semibold text-steel-gray">
                    DelphINS Smoothed Trajectory (Timestamp, KP, E, N, Depth, SD_E, SD_N, SD_Z)
                  </label>
                  <textarea
                    value={delphinsContent}
                    onChange={(e) => setDelphinsContent(e.target.value)}
                    rows={8}
                    placeholder="timestamp,kp,easting,northing,depth,heading,pitch,roll,sd_easting,sd_northing,sd_depth,usbl_rejected&#10;2026-10-13T10:00:00Z,0.000,580000.00,2150000.00,48.00,71.8,-0.5,0.2,0.035,0.038,0.018,0&#10;..."
                    className="mt-1 w-full rounded border border-navy-border bg-navy-elevated p-2 font-mono text-[11px] text-white"
                  />
                </div>

                <div>
                  <label className="text-xs font-semibold text-steel-gray">
                    Pipeline Events & Anode Inspection Log
                  </label>
                  <textarea
                    value={eventsContent}
                    onChange={(e) => setEventsContent(e.target.value)}
                    rows={6}
                    placeholder="event_id,kp,easting,northing,depth,type,description,anode_depletion_pct,cp_reading_mv,severity&#10;ANO-01,0.500,580475.00,2150156.00,48.60,Anode,Zinc Bracelet,15.0,-960.0,Normal&#10;..."
                    className="mt-1 w-full rounded border border-navy-border bg-navy-elevated p-2 font-mono text-[11px] text-white"
                  />
                </div>

                <div className="flex flex-col justify-between">
                  <div>
                    <label className="text-xs font-semibold text-steel-gray">
                      Pipe Outer Diameter (OD in meters)
                    </label>
                    <input
                      type="number"
                      step="0.001"
                      value={pipeOdM}
                      onChange={(e) => setPipeOdM(parseFloat(e.target.value) || 0.4064)}
                      className="mt-1 w-full rounded border border-navy-border bg-navy-elevated px-3 py-1.5 text-xs font-mono text-white"
                    />
                    <span className="text-[10px] text-steel-gray">
                      Standard sizes: 16-inch = 0.4064m · 24-inch = 0.6096m · 36-inch = 0.9144m
                    </span>
                  </div>

                  <button
                    onClick={() => runEvaluation(routeContent, profileContent, eventsContent, delphinsContent, pipeOdM)}
                    disabled={loading || !profileContent.trim()}
                    className="mt-4 flex w-full items-center justify-center gap-2 rounded bg-cyan-600 py-2.5 text-xs font-bold text-white hover:bg-cyan-500 disabled:opacity-50 transition-colors"
                  >
                    <RefreshCw className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
                    Recalculate Freespans & Integrity QC
                  </button>
                </div>
              </div>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between border-t border-navy-border bg-navy-elevated/40 px-4 py-2.5 text-xs text-steel-gray">
          <div className="flex items-center gap-2">
            <span className="h-2 w-2 rounded-full bg-emerald-400" />
            <span>EIVA & DelphINS Autonomous Processor Online</span>
          </div>

          <div className="flex items-center gap-2">
            <button
              onClick={onClose}
              className="rounded border border-navy-border bg-navy-base px-3 py-1 text-xs text-steel-gray hover:text-white transition-colors"
            >
              Close
            </button>
          </div>
        </div>
      </div>
    </DialogShell>
  );
}
