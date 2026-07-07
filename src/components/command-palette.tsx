/**
 * Command Palette — Sprint 2 Priority #4.
 *
 * Ctrl+K opens a fuzzy-search overlay. Type to search across all
 * actions, settings, and CRS codes. Enter executes. Esc closes.
 *
 * On a survey vessel in 2m seas, clicking 16×16px icons is impossible.
 * Typing a command is drastically easier.
 *
 * Recent-commands history: the last 5 executed commands are persisted
 * to localStorage and shown at the top of the empty-state list so the
 * surveyor can one-click back into yesterday's workflow.
 */

import { useEffect, useState, useMemo, useRef } from "react";
import {
  Search, ArrowRight, Calculator, Layers3, Terminal, Shield,
  Waves, Anchor, Brain, History, GitBranch, Settings, Radio,
  FileText, Boxes, Bomb, ShieldAlert, ShieldCheck, Ruler, Package, Scissors,
  Key, Gauge, Activity, FolderOpen, RefreshCw, Package as PackageIcon, Cpu,
  Clock, Crosshair, Grid3x3, SquareDashed, FileSearch, Satellite, Keyboard, Bookmark, Palette, TrendingUp,
} from "lucide-react";
import { colors } from "@/lib/tokens";

export interface CommandAction {
  id: string;
  label: string;
  category: string;
  keywords: string[];
  icon: React.ReactNode;
  action: () => void;
}

interface Props {
  open: boolean;
  onClose: () => void;
  actions: CommandAction[];
}

const RECENT_KEY = "metardu.recent_commands";
const MAX_RECENT = 5;

/** Load recent command ids from localStorage. */
function loadRecentIds(): string[] {
  try {
    const raw = localStorage.getItem(RECENT_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed.filter((x): x is string => typeof x === "string").slice(0, MAX_RECENT);
  } catch {
    return [];
  }
}

/** Persist recent command ids to localStorage. */
function saveRecentIds(ids: string[]): void {
  try {
    localStorage.setItem(RECENT_KEY, JSON.stringify(ids.slice(0, MAX_RECENT)));
  } catch {
    // localStorage may be unavailable — non-fatal
  }
}

/** Push a command id onto the recent list (most-recent-first, deduped). */
function pushRecent(id: string): string[] {
  const next = [id, ...loadRecentIds().filter((x) => x !== id)].slice(0, MAX_RECENT);
  saveRecentIds(next);
  return next;
}

export function CommandPalette({ open, onClose, actions }: Props) {
  const [query, setQuery] = useState("");
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [recentIds, setRecentIds] = useState<string[]>(() => loadRecentIds());
  const inputRef = useRef<HTMLInputElement>(null);

  // Recent actions (resolved from ids) — kept stable across re-renders.
  const recentActions = useMemo(() => {
    if (recentIds.length === 0) return [];
    const byId = new Map(actions.map((a) => [a.id, a]));
    return recentIds
      .map((id) => byId.get(id))
      .filter((a): a is CommandAction => Boolean(a));
  }, [recentIds, actions]);

  // Filter actions by fuzzy match on label + keywords
  const filtered = useMemo(() => {
    if (!query.trim()) {
      // Empty state: show recent first (if any), then top alphabetical.
      // We mark recent ones so the renderer can show a "Recent" header.
      const recents = recentActions;
      const recentIdSet = new Set(recents.map((a) => a.id));
      const rest = actions.filter((a) => !recentIdSet.has(a.id)).slice(0, 10 - recents.length);
      return [...recents, ...rest];
    }
    const q = query.toLowerCase();
    return actions
      .filter((a) => {
        const haystack = (a.label + " " + a.keywords.join(" ") + " " + a.category).toLowerCase();
        // Simple fuzzy: every char in query appears in order
        let qi = 0;
        for (let i = 0; i < haystack.length && qi < q.length; i++) {
          if (haystack[i] === q[qi]) qi++;
        }
        return qi === q.length;
      })
      .slice(0, 12);
  }, [query, actions, recentActions]);

  // Reset on open
  useEffect(() => {
    if (open) {
      setQuery("");
      setSelectedIndex(0);
      setRecentIds(loadRecentIds());
      setTimeout(() => inputRef.current?.focus(), 50);
    }
  }, [open]);

  // Execute a command — also pushes it onto the recent list.
  const runCommand = (action: CommandAction) => {
    setRecentIds(pushRecent(action.id));
    action.action();
    onClose();
  };

  // Keyboard navigation
  useEffect(() => {
    if (!open) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") { onClose(); }
      else if (e.key === "ArrowDown") {
        e.preventDefault();
        setSelectedIndex((i) => Math.min(i + 1, filtered.length - 1));
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        setSelectedIndex((i) => Math.max(i - 1, 0));
      } else if (e.key === "Enter" && filtered[selectedIndex]) {
        e.preventDefault();
        runCommand(filtered[selectedIndex]);
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, filtered, selectedIndex, onClose]);

  if (!open) return null;

  const recentIdSet = new Set(recentActions.map((a) => a.id));

  return (
    <div
      className="fixed inset-0 z-[100] flex items-start justify-center pt-[15vh] bg-black/60 backdrop-blur-sm"
      onClick={onClose}
    >
      <div
        onClick={(e) => e.stopPropagation()}
        className="w-full max-w-xl overflow-hidden rounded-lg border border-navy-border bg-navy-panel shadow-2xl"
        style={{ boxShadow: `0 20px 60px -10px ${colors.industrialOrange}20` }}
      >
        {/* Search input */}
        <div className="flex items-center gap-3 border-b border-navy-border px-4 py-3">
          <Search className="h-4 w-4" style={{ color: colors.industrialOrange }} />
          <input
            ref={inputRef}
            type="text"
            value={query}
            onChange={(e) => { setQuery(e.target.value); setSelectedIndex(0); }}
            placeholder="Type a command… (e.g., 'volume', 's-44', 'epsg')"
            className="flex-1 bg-transparent text-sm text-white placeholder:text-steel-gray focus:outline-none"
          />
          <kbd className="rounded border border-navy-border bg-navy-base px-1.5 py-0.5 text-[9px] font-mono text-steel-gray">
            ESC
          </kbd>
        </div>

        {/* Results */}
        <div className="max-h-[50vh] overflow-y-auto p-2">
          {filtered.length === 0 ? (
            <div className="py-8 text-center text-xs text-steel-gray">
              No commands match "{query}"
            </div>
          ) : (
            filtered.map((action, i) => {
              const isRecent = !query.trim() && recentIdSet.has(action.id);
              const isFirstNonRecent = !query.trim() && !isRecent && i === recentActions.length;
              return (
                <div key={action.id}>
                  {isFirstNonRecent && recentActions.length > 0 && (
                    <div className="px-3 pb-1 pt-2 text-[9px] font-semibold uppercase tracking-wider text-steel-gray">
                      All commands
                    </div>
                  )}
                  <button
                    onClick={() => runCommand(action)}
                    onMouseEnter={() => setSelectedIndex(i)}
                    className={`flex w-full items-center gap-3 rounded-md px-3 py-2 text-left transition-colors ${
                      i === selectedIndex ? "bg-navy-elevated" : "hover:bg-navy-elevated/50"
                    }`}
                    style={i === selectedIndex ? { boxShadow: `inset 2px 0 0 ${colors.industrialOrange}` } : undefined}
                  >
                    <span style={{ color: colors.steelLight }}>{action.icon}</span>
                    <div className="flex-1 min-w-0">
                      <div className="text-sm text-white">{action.label}</div>
                      <div className="text-[10px] text-steel-gray">{action.category}</div>
                    </div>
                    {isRecent && (
                      <span
                        className="flex items-center gap-1 rounded border border-navy-border bg-navy-base px-1.5 py-0.5 text-[8px] font-mono text-steel-gray"
                        title="Recently used"
                      >
                        <Clock className="h-2.5 w-2.5" />
                        RECENT
                      </span>
                    )}
                    {i === selectedIndex && !isRecent && (
                      <ArrowRight className="h-3 w-3" style={{ color: colors.industrialOrange }} />
                    )}
                  </button>
                </div>
              );
            })
          )}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between border-t border-navy-border px-4 py-2 text-[9px] text-steel-gray">
          <span>↑↓ navigate · Enter select · Esc close</span>
          <span>{filtered.length} results{recentActions.length > 0 && !query.trim() ? ` · ${recentActions.length} recent` : ""}</span>
        </div>
      </div>
    </div>
  );
}

/** Helper to create command actions from dialog open callbacks. */
export function createCommandActions(callbacks: {
  onOpenVolumeCalc: () => void;
  onOpenOdm: () => void;
  onOpenCsf: () => void;
  onOpenS44: () => void;
  onOpenCube: () => void;
  onOpenS57: () => void;
  onOpenMl: () => void;
  onOpenMonitoring: () => void;
  onOpenPipeline: () => void;
  onOpenSettings: () => void;
  onToggleProfile: () => void;
  onToggleStream: () => void;
  onOpenEom: () => void;
  onOpenS44Cert: () => void;
  onOpenSvp: () => void;
  onOpenVesselConfig: () => void;
  onOpenCubeDisambig: () => void;
  onOpenDredgeAudit: () => void;
  onOpenStockpileAudit: () => void;
  onOpenBlastReport: () => void;
  onOpenHighwall: () => void;
  onOpenCrossSection: () => void;
  onOpenDeliverable: () => void;
  onOpenSss: () => void;
  onOpenSliceEditor: () => void;
  onOpenLicense: () => void;
  onOpenBenchmark: () => void;
  onOpenTelemetry: () => void;
  onOpenProject: () => void;
  onOpenUpdate: () => void;
  onOpenMarketplace: () => void;
  onOpenDensityGates: () => void;
  onOpenTidalSpline: () => void;
  onOpenMachineControl: () => void;
  onOpenEomAuditor: () => void;
  onOpenTriage: () => void;
  onOpenNtrip: () => void;
  // Sprint 10 — Mining field tools
  onOpenSetout: () => void;
  onOpenMineGrid: () => void;
  onOpenTunnelProfile: () => void;
  onOpenSafetyReport: () => void;
  // Sprint 10 — Marine field tools
  onOpenTidalDatum: () => void;
  onOpenBackscatter: () => void;
  onOpenQcDashboard: () => void;
  onOpenMbesSurvey: () => void;
  // Sprint 10 — Stockpile change detection
  onOpenStockpileChange: () => void;
  // Sprint 11 — Real-time field ops
  onOpenRoverStream: () => void;
  onOpenTideGauge: () => void;
  // Sprint 12 — UI polish
  onOpenShortcuts: () => void;
  // Sprint 13 — UI priorities
  onOpenSavedViews: () => void;
  onOpenCustomizeToolbar: () => void;
  onToggleTheme: () => void;
  // Sprint 16 — GIS gap features
  onOpenIdw: () => void;
  onOpenShapefile: () => void;
  onOpenTopology: () => void;
  // Sprint 17
  onOpenMapLayout: () => void;
  onToggleColorblind: () => void;
}): CommandAction[] {
  const icon = "h-4 w-4";
  return [
    { id: "eom", label: "EoM Reconciliation Wizard", category: "Mining · Revenue", keywords: ["reconciliation", "volume", "report", "mining", "monthly"], icon: <Calculator className={icon} />, action: callbacks.onOpenEom },
    { id: "stockpile", label: "Stockpile Inventory Audit", category: "Mining · Revenue", keywords: ["stockpile", "inventory", "tonnage", "audit", "rom", "pad"], icon: <Boxes className={icon} />, action: callbacks.onOpenStockpileAudit },
    { id: "blast", label: "Blast Fragmentation Report", category: "Mining · Revenue", keywords: ["blast", "fragmentation", "p80", "p50", "muck", "powder"], icon: <Bomb className={icon} />, action: callbacks.onOpenBlastReport },
    { id: "highwall", label: "Highwall Deformation Monitoring", category: "Mining · Revenue", keywords: ["highwall", "slope", "deformation", "displacement", "alert", "compliance", "safety"], icon: <ShieldAlert className={icon} />, action: callbacks.onOpenHighwall },
    { id: "dredge", label: "Dredge Pay-Volume Audit", category: "Marine · Revenue", keywords: ["dredge", "pay", "overdredge", "shoaling", "channel", "port"], icon: <Waves className={icon} />, action: callbacks.onOpenDredgeAudit },
    { id: "xsec", label: "Cross-Section Profiler", category: "Marine · Revenue", keywords: ["cross", "section", "profile", "channel", "design", "dredge", "verify"], icon: <Ruler className={icon} />, action: callbacks.onOpenCrossSection },
    { id: "deliverable", label: "Survey Deliverable Package", category: "Marine · Revenue", keywords: ["deliverable", "package", "zip", "manifest", "metadata", "iso", "19115", "bundle"], icon: <Package className={icon} />, action: callbacks.onOpenDeliverable },
    { id: "sss", label: "SSS Waterfall Viewer", category: "Marine · Advanced", keywords: ["sss", "side", "scan", "sonar", "waterfall", "xtf", "backscatter", "shadow"], icon: <Waves className={icon} />, action: callbacks.onOpenSss },
    { id: "slice", label: "3D Slice Editor (Reject Brush)", category: "Cross-cutting · Advanced", keywords: ["slice", "3d", "polygon", "reject", "brush", "cube", "clean", "qc"], icon: <Scissors className={icon} />, action: callbacks.onOpenSliceEditor },
    { id: "license", label: "License Manager", category: "Enterprise · Activation", keywords: ["license", "activate", "pro", "enterprise", "trial", "tier", "unlock"], icon: <Key className={icon} />, action: callbacks.onOpenLicense },
    { id: "benchmark", label: "Performance Benchmark Suite", category: "Enterprise · Diagnostics", keywords: ["benchmark", "performance", "speed", "timing", "throughput", "cpu"], icon: <Gauge className={icon} />, action: callbacks.onOpenBenchmark },
    { id: "telemetry", label: "Telemetry & Crash Reporter", category: "Enterprise · Diagnostics", keywords: ["telemetry", "crash", "report", "usage", "stats", "diagnostics", "privacy"], icon: <Activity className={icon} />, action: callbacks.onOpenTelemetry },
    { id: "project", label: "Project Manager", category: "File · Project", keywords: ["project", "save", "load", "open", "new", "metardu"], icon: <FolderOpen className={icon} />, action: callbacks.onOpenProject },
    { id: "update", label: "Check for Updates", category: "App · Updates", keywords: ["update", "version", "upgrade", "download", "release"], icon: <RefreshCw className={icon} />, action: callbacks.onOpenUpdate },
    { id: "marketplace", label: "Plugin Marketplace", category: "Enterprise · Plugins", keywords: ["plugin", "marketplace", "install", "browse", "registry", "extension"], icon: <PackageIcon className={icon} />, action: callbacks.onOpenMarketplace },
    { id: "density_gates", label: "Density Gates (Coverage Validator)", category: "Marine · Bottleneck", keywords: ["density", "coverage", "gap", "survey", "s44", "iho", "quality", "qc"], icon: <Activity className={icon} />, action: callbacks.onOpenDensityGates },
    { id: "tidal_spline", label: "Tidal Spline Corrector", category: "Marine · Bottleneck", keywords: ["tide", "tidal", "spline", "correction", "depth", "sonar", "gauge", "interpolate"], icon: <Waves className={icon} />, action: callbacks.onOpenTidalSpline },
    { id: "machine_control", label: "Machine Control Compiler", category: "Mining · Bottleneck", keywords: ["machine", "control", "dxf", "leica", "trimble", "topcon", "svd", "tp3", "guidance", "dozer"], icon: <Cpu className={icon} />, action: callbacks.onOpenMachineControl },
    { id: "eom_auditor", label: "EOM Volumetric Auditor", category: "Mining · Revenue", keywords: ["eom", "end", "month", "volumetric", "audit", "reconcile", "production", "report", "pdf", "license"], icon: <ShieldCheck className={icon} />, action: callbacks.onOpenEomAuditor },
    { id: "triage", label: "Mission Data Triage", category: "Field Tools", keywords: ["triage", "field", "verify", "coverage", "gap", "exif", "gnss"], icon: <FolderOpen className={icon} />, action: callbacks.onOpenTriage },
    { id: "ntrip", label: "NTRIP Client", category: "Field Tools", keywords: ["ntrip", "rtcm", "rtk", "correction", "gnss", "base", "station"], icon: <Radio className={icon} />, action: callbacks.onOpenNtrip },
    { id: "volume", label: "Volume Calculator", category: "Mining", keywords: ["volume", "fill", "cut", "bench", "stockpile"], icon: <Calculator className={icon} />, action: callbacks.onOpenVolumeCalc },
    { id: "odm", label: "ODM Pipeline (Drone → Point Cloud)", category: "Mining", keywords: ["odm", "drone", "photogrammetry", "docker"], icon: <Terminal className={icon} />, action: callbacks.onOpenOdm },
    { id: "csf", label: "Classify Ground (CSF)", category: "Mining", keywords: ["classify", "ground", "csf", "cloth", "point cloud"], icon: <Layers3 className={icon} />, action: callbacks.onOpenCsf },
    { id: "monitoring", label: "4D Pit Monitoring", category: "Mining", keywords: ["4d", "monitoring", "deformation", "highwall", "slope"], icon: <History className={icon} />, action: callbacks.onOpenMonitoring },
    { id: "cube", label: "CUBE Surface Generation", category: "Marine", keywords: ["cube", "bathymetry", "surface", "marine"], icon: <Waves className={icon} />, action: callbacks.onOpenCube },
    { id: "s44", label: "S-44 Compliance Check", category: "Marine", keywords: ["s-44", "iho", "compliance", "hydrographic"], icon: <Shield className={icon} />, action: callbacks.onOpenS44 },
    { id: "s44cert", label: "S-44 Compliance Certificate", category: "Marine · Revenue", keywords: ["s-44", "certificate", "report", "compliance"], icon: <FileText className={icon} />, action: callbacks.onOpenS44Cert },
    { id: "s57", label: "S-57 Export", category: "Marine", keywords: ["s-57", "enc", "export", "wreck", "obstruction"], icon: <Anchor className={icon} />, action: callbacks.onOpenS57 },
    { id: "ml", label: "ML Classification", category: "Cross-cutting", keywords: ["ml", "habitat", "fragmentation", "blast", "seafloor"], icon: <Brain className={icon} />, action: callbacks.onOpenMl },
    { id: "pipeline", label: "Automation — Pipelines", category: "Cross-cutting", keywords: ["pipeline", "automation", "watch", "schedule", "yaml"], icon: <GitBranch className={icon} />, action: callbacks.onOpenPipeline },
    { id: "profile", label: "Toggle Profile Tool", category: "Map", keywords: ["profile", "elevation", "cross-section", "dem"], icon: <TrendingUp className={icon} />, action: callbacks.onToggleProfile },
    { id: "stream", label: "Toggle Live Stream (UDP)", category: "Map", keywords: ["stream", "udp", "live", "real-time", "sonar"], icon: <Radio className={icon} />, action: callbacks.onToggleStream },
    { id: "settings", label: "Settings", category: "App", keywords: ["settings", "theme", "epsg", "crs", "density"], icon: <Settings className={icon} />, action: callbacks.onOpenSettings },
    { id: "svp", label: "SVP Editor (Sound Velocity)", category: "Marine", keywords: ["svp", "sound", "velocity", "profile", "ray", "tracing"], icon: <Waves className={icon} />, action: callbacks.onOpenSvp },
    { id: "vessel", label: "Vessel Configuration (Lever-Arms)", category: "Marine", keywords: ["vessel", "lever", "arm", "offset", "imu", "transducer", "gnss", "tpu"], icon: <Anchor className={icon} />, action: callbacks.onOpenVesselConfig },
    { id: "disambig", label: "CUBE Hypothesis Disambiguation", category: "Marine", keywords: ["cube", "hypothesis", "disambiguation", "ambiguous", "qc"], icon: <Layers3 className={icon} />, action: callbacks.onOpenCubeDisambig },
    // ── Sprint 10: Mining field tools ──
    { id: "setout", label: "Setting Out & Markout", category: "Mining · Field Tools", keywords: ["setout", "markout", "bearing", "distance", "blast", "peg", "total station", "rtk"], icon: <Crosshair className={icon} />, action: callbacks.onOpenSetout },
    { id: "mine_grid", label: "Mine Grid Transform", category: "Mining · Field Tools", keywords: ["mine", "grid", "transform", "rotation", "scale", "crs", "local"], icon: <Grid3x3 className={icon} />, action: callbacks.onOpenMineGrid },
    { id: "tunnel_profile", label: "Tunnel Profile Analyzer", category: "Mining · Field Tools", keywords: ["tunnel", "profile", "overbreak", "underbreak", "drive", "cross-section", "area"], icon: <SquareDashed className={icon} />, action: callbacks.onOpenTunnelProfile },
    { id: "safety_report", label: "Safety Inspection Report", category: "Mining · Field Tools", keywords: ["safety", "inspection", "hazard", "compliance", "regulator", "risk"], icon: <ShieldAlert className={icon} />, action: callbacks.onOpenSafetyReport },
    // ── Sprint 10: Marine field tools ──
    { id: "tidal_datum", label: "Tidal Datum Converter", category: "Marine · Field Tools", keywords: ["tidal", "datum", "mllw", "msl", "cd", "lat", "navd88", "convert", "depth"], icon: <Waves className={icon} />, action: callbacks.onOpenTidalDatum },
    { id: "backscatter", label: "Backscatter Mosaic Builder", category: "Marine · Field Tools", keywords: ["backscatter", "mosaic", "intensity", "lambert", "seabed", "classification"], icon: <Grid3x3 className={icon} />, action: callbacks.onOpenBackscatter },
    { id: "qc_dashboard", label: "Real-Time QC Dashboard", category: "Marine · Field Tools", keywords: ["qc", "quality", "control", "s44", "density", "coverage", "compliance", "uncertainty"], icon: <Activity className={icon} />, action: callbacks.onOpenQcDashboard },
    { id: "mbes_survey", label: "MBES Survey Reader (Kongsberg .all)", category: "Marine · Field Tools", keywords: ["mbes", "kongsberg", "all", "bathymetry", "multibeam", "ingest", "sonar"], icon: <FileSearch className={icon} />, action: callbacks.onOpenMbesSurvey },
    // ── Sprint 10: Volumetric change detection ──
    { id: "stockpile_change", label: "Stockpile Change Detection (Cut/Fill)", category: "Mining · Revenue", keywords: ["stockpile", "change", "cut", "fill", "delta", "epoch", "reconcile", "progress"], icon: <History className={icon} />, action: callbacks.onOpenStockpileChange },
    // ── Sprint 11: Real-time field ops ──
    { id: "rover_stream", label: "RTK Rover Stream (NMEA over TCP)", category: "Field Tools · Real-time", keywords: ["rover", "rtk", "gnss", "nmea", "tcp", "position", "gga", "rmc", "live", "stream"], icon: <Satellite className={icon} />, action: callbacks.onOpenRoverStream },
    { id: "tide_gauge", label: "Tide Gauge (NOAA CO-OPS / TCP)", category: "Marine · Real-time", keywords: ["tide", "gauge", "noaa", "co-ops", "water", "level", "mlw", "correction", "bathymetry", "live"], icon: <Waves className={icon} />, action: callbacks.onOpenTideGauge },
    // ── Sprint 12: UI polish ──
    { id: "shortcuts", label: "Keyboard Shortcuts Help", category: "App · Help", keywords: ["keyboard", "shortcuts", "help", "hotkey", "cheatsheet", "?"], icon: <Keyboard className={icon} />, action: callbacks.onOpenShortcuts },
    // ── Sprint 13: UI priorities ──
    { id: "saved_views", label: "Saved Views", category: "App · Map", keywords: ["saved", "view", "bookmark", "extent", "zoom", "restore", "snapshot"], icon: <Bookmark className={icon} />, action: callbacks.onOpenSavedViews },
    { id: "customize_toolbar", label: "Customize Toolbar", category: "App · UI", keywords: ["customize", "toolbar", "pin", "actions", "shortcuts", "favorites"], icon: <Settings className={icon} />, action: callbacks.onOpenCustomizeToolbar },
    { id: "toggle_theme", label: "Toggle Theme (Dark/Light)", category: "App · UI", keywords: ["theme", "dark", "light", "daylight", "cabin", "toggle", "mode"], icon: <Palette className={icon} />, action: callbacks.onToggleTheme },
    // ── Sprint 16: GIS gap features ──
    { id: "idw", label: "IDW Interpolation", category: "GIS Tools · Interpolation", keywords: ["idw", "interpolation", "inverse", "distance", "weighting", "dem", "gap", "fill", "surface", "grid"], icon: <TrendingUp className={icon} />, action: callbacks.onOpenIdw },
    { id: "shapefile", label: "Shapefile Import", category: "GIS Tools · Import", keywords: ["shapefile", "shp", "esri", "import", "load", "overlay", "surpac", "datamine", "vulcan"], icon: <FileSearch className={icon} />, action: callbacks.onOpenShapefile },
    { id: "topology", label: "Topology Validator", category: "GIS Tools · QA", keywords: ["topology", "validate", "quality", "qa", "qc", "gap", "overlap", "dangle", "sliver", "self-intersection"], icon: <ShieldCheck className={icon} />, action: callbacks.onOpenTopology },
    // ── Sprint 17 ──
    { id: "map_layout", label: "Generate Map Sheet (PDF)", category: "GIS Tools · Export", keywords: ["map", "layout", "sheet", "pdf", "print", "title", "block", "north", "arrow", "scale", "legend"], icon: <FileText className={icon} />, action: callbacks.onOpenMapLayout },
    { id: "colorblind", label: "Toggle Colorblind Palette", category: "App · Accessibility", keywords: ["colorblind", "colour", "blind", "palette", "accessibility", "wcag", "deuteranopia", "protanopia"], icon: <Palette className={icon} />, action: callbacks.onToggleColorblind },
  ];
}
