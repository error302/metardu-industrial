/**
 * Project Templates — Sprint 11 Quality of Life #1.
 *
 * Predefined project templates that pre-load the right combination of
 * dialogs and settings for common workflows. Eliminates the
 * "what do I click first?" friction for new operators and standardizes
 * workflows across a survey team.
 *
 * Templates:
 *   - Stockpile Audit (Mining): Stockpile Audit wizard + Volume + Change Detection
 *   - Dredge Audit (Marine): Dredge wizard + Cross-Section + S-44 Compliance
 *   - EOM Reconciliation (Mining): EOM Auditor + EOM Wizard + Benchmark
 *   - Bathymetric Survey (Marine): MBES Reader + QC Dashboard + SVP + Vessel
 *   - Blank Project: no presets
 *
 * Each template specifies the domain, default EPSG, dialogs to open
 * (by a string key the workspace shell resolves to its `set<Dialog>Open(true)`
 * callback), and a suggested project name prefix.
 */

import type { DomainMode } from "@/lib/tokens";

export type DialogKey =
  // Mining dialogs
  | "stockpileAudit" | "volumeCalc" | "stockpileChange" | "blastReport"
  | "highwall" | "monitoring" | "eomAuditor" | "eom" | "csf" | "odm"
  | "machineControl" | "setout" | "mineGrid" | "tunnelProfile" | "safetyReport"
  // Marine dialogs
  | "dredgeAudit" | "crossSection" | "s44" | "s44Cert" | "s57"
  | "cube" | "cubeDisambig" | "svp" | "vesselConfig" | "sss"
  | "densityGates" | "tidalSpline" | "tidalDatum" | "backscatter"
  | "qcDashboard" | "mbesSurvey"
  // Cross-cutting
  | "ml" | "pipeline" | "deliverable" | "sliceEditor" | "benchmark"
  | "ntrip" | "roverStream" | "tideGauge";

export interface ProjectTemplate {
  id: string;
  name: string;
  description: string;
  icon: string; // emoji or icon name — kept simple for the picker
  domain: DomainMode;
  defaultEpsg: string;
  defaultDensity: "compact" | "comfortable";
  /** Dialogs to auto-open after project creation (in order). */
  dialogsToOpen: DialogKey[];
  /** Suggested name prefix for the new project. */
  namePrefix: string;
}

export const PROJECT_TEMPLATES: ProjectTemplate[] = [
  {
    id: "blank",
    name: "Blank Project",
    description: "Start from scratch with no presets. Best for advanced users or custom workflows.",
    icon: "○",
    domain: "both",
    defaultEpsg: "EPSG:3857",
    defaultDensity: "comfortable",
    dialogsToOpen: [],
    namePrefix: "Project",
  },
  {
    id: "stockpile-audit",
    name: "Stockpile Audit",
    description: "Drone-survey a stockpile yard, compute tonnage, compare to previous survey. Mining domain.",
    icon: "△",
    domain: "mining",
    defaultEpsg: "EPSG:28355", // MGA Zone 55 — common Australian mining CRS
    defaultDensity: "comfortable",
    dialogsToOpen: ["stockpileAudit", "volumeCalc", "stockpileChange"],
    namePrefix: "Stockpile",
  },
  {
    id: "dredge-audit",
    name: "Dredge Audit",
    description: "Pre/post-dredge survey comparison with pay-volume and cross-section compliance. Marine domain.",
    icon: "⚓",
    domain: "marine",
    defaultEpsg: "EPSG:32756", // UTM Zone 56S — common port authority
    defaultDensity: "comfortable",
    dialogsToOpen: ["dredgeAudit", "crossSection", "s44"],
    namePrefix: "Dredge",
  },
  {
    id: "eom-reconciliation",
    name: "EOM Reconciliation",
    description: "End-of-month production reconciliation — LAS → signed PDF volume report. Mining domain.",
    icon: "∑",
    domain: "mining",
    defaultEpsg: "EPSG:28355",
    defaultDensity: "compact",
    dialogsToOpen: ["eomAuditor", "eom", "benchmark"],
    namePrefix: "EOM",
  },
  {
    id: "bathymetric-survey",
    name: "Bathymetric Survey",
    description: "Multibeam survey ingest, real-time QC, CUBE surface generation, S-44 certificate. Marine domain.",
    icon: "≈",
    domain: "marine",
    defaultEpsg: "EPSG:32756",
    defaultDensity: "comfortable",
    dialogsToOpen: ["mbesSurvey", "qcDashboard", "svp", "vesselConfig"],
    namePrefix: "Bathy",
  },
  {
    id: "highwall-monitoring",
    name: "Highwall Monitoring",
    description: "Slope-stability time-series monitoring with USACE-compliant alerts and PDF report. Mining domain.",
    icon: "⌒",
    domain: "mining",
    defaultEpsg: "EPSG:28355",
    defaultDensity: "comfortable",
    dialogsToOpen: ["highwall", "monitoring"],
    namePrefix: "Highwall",
  },
];

/**
 * Resolve a template's `dialogsToOpen` keys to a list of friendly
 * labels for display in the template picker ("Opens: Stockpile Audit,
 * Volume Calculator, Change Detection").
 */
export function dialogLabels(keys: DialogKey[]): string[] {
  const labels: Record<DialogKey, string> = {
    stockpileAudit: "Stockpile Audit",
    volumeCalc: "Volume Calculator",
    stockpileChange: "Change Detection",
    blastReport: "Blast Report",
    highwall: "Highwall Monitoring",
    monitoring: "4D Monitoring",
    eomAuditor: "EOM Auditor",
    eom: "EOM Wizard",
    csf: "Classify Ground (CSF)",
    odm: "ODM Photogrammetry",
    machineControl: "Machine Control",
    setout: "Setting Out",
    mineGrid: "Mine Grid",
    tunnelProfile: "Tunnel Profile",
    safetyReport: "Safety Report",
    dredgeAudit: "Dredge Audit",
    crossSection: "Cross-Section Profiler",
    s44: "S-44 Compliance",
    s44Cert: "S-44 Certificate",
    s57: "S-57 Export",
    cube: "CUBE Surface",
    cubeDisambig: "CUBE Disambiguation",
    svp: "SVP Editor",
    vesselConfig: "Vessel Configuration",
    sss: "SSS Waterfall",
    densityGates: "Density Gates",
    tidalSpline: "Tidal Spline",
    tidalDatum: "Tidal Datum Converter",
    backscatter: "Backscatter Mosaic",
    qcDashboard: "QC Dashboard",
    mbesSurvey: "MBES Survey Reader",
    ml: "ML Classification",
    pipeline: "Pipeline Editor",
    deliverable: "Deliverable Package",
    sliceEditor: "3D Slice Editor",
    benchmark: "Benchmark",
    ntrip: "NTRIP Client",
    roverStream: "RTK Rover Stream",
    tideGauge: "Tide Gauge",
  };
  return keys.map(k => labels[k] ?? k);
}
