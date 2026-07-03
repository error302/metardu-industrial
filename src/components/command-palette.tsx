/**
 * Command Palette — Sprint 2 Priority #4.
 *
 * Ctrl+K opens a fuzzy-search overlay. Type to search across all
 * actions, settings, and CRS codes. Enter executes. Esc closes.
 *
 * On a survey vessel in 2m seas, clicking 16×16px icons is impossible.
 * Typing a command is drastically easier.
 */

import { useEffect, useState, useMemo, useRef } from "react";
import {
  Search, ArrowRight, Calculator, Layers3, Terminal, Shield,
  Waves, Anchor, Brain, History, GitBranch, Settings, Radio,
  TrendingUp, FileText, Boxes, Bomb, ShieldAlert, Ruler, Package, Scissors,
  Key, Gauge, Activity, FolderOpen, RefreshCw, Package as PackageIcon, Cpu,
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

export function CommandPalette({ open, onClose, actions }: Props) {
  const [query, setQuery] = useState("");
  const [selectedIndex, setSelectedIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  // Filter actions by fuzzy match on label + keywords
  const filtered = useMemo(() => {
    if (!query.trim()) return actions.slice(0, 10);
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
  }, [query, actions]);

  // Reset on open
  useEffect(() => {
    if (open) {
      setQuery("");
      setSelectedIndex(0);
      setTimeout(() => inputRef.current?.focus(), 50);
    }
  }, [open]);

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
        filtered[selectedIndex].action();
        onClose();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [open, filtered, selectedIndex, onClose]);

  if (!open) return null;

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
            filtered.map((action, i) => (
              <button
                key={action.id}
                onClick={() => { action.action(); onClose(); }}
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
                {i === selectedIndex && (
                  <ArrowRight className="h-3 w-3" style={{ color: colors.industrialOrange }} />
                )}
              </button>
            ))
          )}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between border-t border-navy-border px-4 py-2 text-[9px] text-steel-gray">
          <span>↑↓ navigate · Enter select · Esc close</span>
          <span>{filtered.length} results</span>
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
  onOpenDensityGates: () => void;
  onOpenTidalSpline: () => void;
  onOpenMachineControl: () => void;
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
  ];
}
