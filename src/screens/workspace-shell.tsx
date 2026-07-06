/**
 * Workspace Shell — main application frame.
 *
 * Layout per ARCHITECTURE.md §6:
 *   - Title bar with brand mark + workspace name + window controls
 *   - Left sidebar: project tree / data tree
 *   - Center: OpenLayers map canvas (primary)
 *   - Right sidebar: contextual panel (volume calc / S-44 status / etc.)
 *   - Bottom: status bar (CRS, domain, GNSS fix, module status)
 *
 * Color mode shifts based on activeDomain (mining/marine/both).
 */

import { useCallback, useEffect, useMemo, useState } from "react";
import type Map from "ol/Map";
import { useViewport } from "@/lib/use-viewport";
import { isNative } from "@/lib/tauri-ipc";
import {
  Folder,
  FileBox,
  Layers,
  Settings,
  HelpCircle,
  Minus,
  Square,
  X,
  Activity,
  MapPin,
  Crosshair,
  Clock,
  ChevronRight,
  Plus,
  TrendingUp,
  Calculator,
  Layers3,
  Terminal,
  Shield,
  Waves,
  Anchor,
  Brain,
  History,
  GitBranch,
  FolderOpen,
  Radio,
  Boxes,
  Bomb,
  ShieldAlert,
  ShieldCheck,
  Ruler,
  Package,
  Key,
  Gauge,
  RefreshCw,
  Cpu,
  Scissors,
  PanelLeft,
  PanelRight,
  Grid3x3,
  SquareDashed,
  FileSearch,
  Satellite,
} from "lucide-react";
import { MapCanvas } from "@/components/map-canvas";
import { FileDropOverlay } from "@/components/file-drop-overlay";
import { CrsSwitchBanner } from "@/components/crs-switch-banner";
import { BrandLogoMark } from "@/components/brand-logo";
import { ProfilePanel } from "@/components/profile-panel";
import { CubeSurfaceOverlay } from "@/components/cube-surface-overlay";
import {
  LayoutProfiles,
  getLayoutSettings,
  loadPersistedLayout,
  type LayoutProfile,
} from "@/components/layout-profiles";
import { createCommandActions, CommandPalette } from "@/components/command-palette";
import { PointCloudLayer, type StreamPing } from "@/components/point-cloud-layer";
// Note: PointCloudLayer is kept as a static import because it's used
// conditionally (only when activeFileId is set) but its import is
// tree-shakeable — Vite already splits deck.gl into a separate vendor
// chunk (deckgl-vendor-*.js, 582KB) that's only fetched when the
// component first renders. No code change needed here.
import { LiveStreamPanel } from "@/components/live-stream-panel";
// ── Lazy-loaded dialogs ──
// Each dialog is a separate chunk loaded on first open. This cuts
// ~150-250KB off the initial bundle and speeds up first paint. The
// <Suspense fallback={null}> wrapper below means the dialog renders
// nothing until its chunk loads — which is fine because dialogs are
// only rendered when `open` is true, so the user clicked to open it
// and a 50-100ms load delay is imperceptible.
import { lazy, Suspense } from "react";
const SettingsDialog = lazy(() => import("@/components/settings-dialog").then(m => ({ default: m.SettingsDialog })));
const VolumeCalcDialog = lazy(() => import("@/components/volume-calc-dialog").then(m => ({ default: m.VolumeCalcDialog })));
const OdmPipelineDialog = lazy(() => import("@/components/odm-pipeline-dialog").then(m => ({ default: m.OdmPipelineDialog })));
const CsfClassificationDialog = lazy(() => import("@/components/csf-classification-dialog").then(m => ({ default: m.CsfClassificationDialog })));
const S44ComplianceDialog = lazy(() => import("@/components/s44-compliance-dialog").then(m => ({ default: m.S44ComplianceDialog })));
const CubeSurfaceDialog = lazy(() => import("@/components/cube-surface-dialog").then(m => ({ default: m.CubeSurfaceDialog })));
const S57ExportDialog = lazy(() => import("@/components/s57-export-dialog").then(m => ({ default: m.S57ExportDialog })));
const Monitoring4DDialog = lazy(() => import("@/components/monitoring-4d-dialog").then(m => ({ default: m.Monitoring4DDialog })));
const MlClassificationDialog = lazy(() => import("@/components/ml-classification-dialog").then(m => ({ default: m.MlClassificationDialog })));
const PipelineEditorDialog = lazy(() => import("@/components/pipeline-editor-dialog").then(m => ({ default: m.PipelineEditorDialog })));
const EomReconciliationWizard = lazy(() => import("@/components/eom-reconciliation-wizard").then(m => ({ default: m.EomReconciliationWizard })));
const S44CertificateDialog = lazy(() => import("@/components/s44-certificate-dialog").then(m => ({ default: m.S44CertificateDialog })));
const SvpEditorDialog = lazy(() => import("@/components/svp-editor-dialog").then(m => ({ default: m.SvpEditorDialog })));
const VesselConfigDialog = lazy(() => import("@/components/vessel-config-dialog").then(m => ({ default: m.VesselConfigDialog })));
const CubeDisambiguationDialog = lazy(() => import("@/components/cube-disambiguation-dialog").then(m => ({ default: m.CubeDisambiguationDialog })));
const DredgeAuditWizard = lazy(() => import("@/components/dredge-audit-wizard").then(m => ({ default: m.DredgeAuditWizard })));
const StockpileAuditWizard = lazy(() => import("@/components/stockpile-audit-wizard").then(m => ({ default: m.StockpileAuditWizard })));
const BlastReportWizard = lazy(() => import("@/components/blast-report-wizard").then(m => ({ default: m.BlastReportWizard })));
const HighwallMonitoringWizard = lazy(() => import("@/components/highwall-monitoring-wizard").then(m => ({ default: m.HighwallMonitoringWizard })));
const CrossSectionProfilerWizard = lazy(() => import("@/components/cross-section-profiler-wizard").then(m => ({ default: m.CrossSectionProfilerWizard })));
const DeliverablePackageWizard = lazy(() => import("@/components/deliverable-package-wizard").then(m => ({ default: m.DeliverablePackageWizard })));
const EomAuditorDialog = lazy(() => import("@/components/eom-auditor-dialog").then(m => ({ default: m.EomAuditorDialog })));
const TriageDialog = lazy(() => import("@/components/triage-dialog").then(m => ({ default: m.TriageDialog })));
const NtripDialog = lazy(() => import("@/components/ntrip-dialog").then(m => ({ default: m.NtripDialog })));
const SssWaterfallViewer = lazy(() => import("@/components/sss-waterfall-viewer").then(m => ({ default: m.SssWaterfallViewer })));
const SliceEditor3D = lazy(() => import("@/components/slice-editor-3d").then(m => ({ default: m.SliceEditor3D })));
const LicenseManagerDialog = lazy(() => import("@/components/license-manager-dialog").then(m => ({ default: m.LicenseManagerDialog })));
const BenchmarkDialog = lazy(() => import("@/components/benchmark-dialog").then(m => ({ default: m.BenchmarkDialog })));
const TelemetryDialog = lazy(() => import("@/components/telemetry-dialog").then(m => ({ default: m.TelemetryDialog })));
const ProjectManagerDialog = lazy(() => import("@/components/project-manager-dialog").then(m => ({ default: m.ProjectManagerDialog })));
const UpdateCheckerDialog = lazy(() => import("@/components/update-checker-dialog").then(m => ({ default: m.UpdateCheckerDialog })));
const PluginMarketplaceDialog = lazy(() => import("@/components/plugin-marketplace-dialog").then(m => ({ default: m.PluginMarketplaceDialog })));
const DensityGatesTool = lazy(() => import("@/components/density-gates-tool").then(m => ({ default: m.DensityGatesTool })));
const TidalSplineTool = lazy(() => import("@/components/tidal-spline-tool").then(m => ({ default: m.TidalSplineTool })));
const MachineControlTool = lazy(() => import("@/components/machine-control-tool").then(m => ({ default: m.MachineControlTool })));
// Sprint 10 — Mining field tools
const SetoutToolDialog = lazy(() => import("@/components/setout-tool-dialog").then(m => ({ default: m.SetoutToolDialog })));
const MineGridDialog = lazy(() => import("@/components/mine-grid-dialog").then(m => ({ default: m.MineGridDialog })));
const TunnelProfileDialog = lazy(() => import("@/components/tunnel-profile-dialog").then(m => ({ default: m.TunnelProfileDialog })));
const SafetyReportDialog = lazy(() => import("@/components/safety-report-dialog").then(m => ({ default: m.SafetyReportDialog })));
// Sprint 10 — Marine field tools
const TidalDatumDialog = lazy(() => import("@/components/tidal-datum-dialog").then(m => ({ default: m.TidalDatumDialog })));
const BackscatterMosaicDialog = lazy(() => import("@/components/backscatter-mosaic-dialog").then(m => ({ default: m.BackscatterMosaicDialog })));
const QcDashboardDialog = lazy(() => import("@/components/qc-dashboard-dialog").then(m => ({ default: m.QcDashboardDialog })));
const MbesSurveyDialog = lazy(() => import("@/components/mbes-survey-dialog").then(m => ({ default: m.MbesSurveyDialog })));
// Sprint 10 — Stockpile change detection
const StockpileChangeDialog = lazy(() => import("@/components/stockpile-change-dialog").then(m => ({ default: m.StockpileChangeDialog })));
// Sprint 11 — Real-time field ops + QoL
const RoverStreamDialog = lazy(() => import("@/components/rover-stream-dialog").then(m => ({ default: m.RoverStreamDialog })));
const TideGaugeDialog = lazy(() => import("@/components/tide-gauge-dialog").then(m => ({ default: m.TideGaugeDialog })));
import { useProfileTool, type ProfileLine } from "@/lib/use-profile-tool";
import type { CsfResult, CubeSurfaceRpc, MetarduProject } from "@/lib/tauri-ipc";
import { startStream, stopStream } from "@/lib/tauri-ipc";
import { useUndoStore } from "@/stores/undo-store";
import type { DialogKey } from "@/lib/project-templates";
import {
  colors,
  domainAccent,
  APP_VERSION,
  type DomainMode,
} from "@/lib/tokens";
import { useAppStore } from "@/stores/app-store";
import { useSurveyStore } from "@/stores/survey-store";

export function WorkspaceShell() {
  const { activeDomain, settings } = useAppStore();
  const viewport = useViewport();
  const [sidebarOpen, setSidebarOpen] = useState(true);
  const [rightPanelOpen, setRightPanelOpen] = useState(true);
  // Tracks whether the sidebar is in overlay drawer mode (very narrow widths).
  // Below sm, the sidebar slides in over the map rather than pushing the map.
  const [drawerSidebarOpen, setDrawerSidebarOpen] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [volumeCalcOpen, setVolumeCalcOpen] = useState(false);
  const [odmOpen, setOdmOpen] = useState(false);
  const [csfOpen, setCsfOpen] = useState(false);
  const [s44Open, setS44Open] = useState(false);
  const [cubeOpen, setCubeOpen] = useState(false);
  const [s57Open, setS57Open] = useState(false);
  const [monitoringOpen, setMonitoringOpen] = useState(false);
  const [mlOpen, setMlOpen] = useState(false);
  const [pipelineOpen, setPipelineOpen] = useState(false);
  const [eomOpen, setEomOpen] = useState(false);
  const [s44CertOpen, setS44CertOpen] = useState(false);
  const [commandPaletteOpen, setCommandPaletteOpen] = useState(false);
  const [svpOpen, setSvpOpen] = useState(false);
  const [vesselConfigOpen, setVesselConfigOpen] = useState(false);
  const [cubeDisambigOpen, setCubeDisambigOpen] = useState(false);
  const [dredgeAuditOpen, setDredgeAuditOpen] = useState(false);
  const [stockpileAuditOpen, setStockpileAuditOpen] = useState(false);
  const [blastReportOpen, setBlastReportOpen] = useState(false);
  const [highwallOpen, setHighwallOpen] = useState(false);
  const [crossSectionOpen, setCrossSectionOpen] = useState(false);
  const [deliverableOpen, setDeliverableOpen] = useState(false);
  const [sssOpen, setSssOpen] = useState(false);
  const [sliceEditorOpen, setSliceEditorOpen] = useState(false);
  const [licenseOpen, setLicenseOpen] = useState(false);
  const [benchmarkOpen, setBenchmarkOpen] = useState(false);
  const [telemetryOpen, setTelemetryOpen] = useState(false);
  const [projectOpen, setProjectOpen] = useState(false);
  const [updateOpen, setUpdateOpen] = useState(false);
  const [marketplaceOpen, setMarketplaceOpen] = useState(false);
  const [densityGatesOpen, setDensityGatesOpen] = useState(false);
  const [tidalSplineOpen, setTidalSplineOpen] = useState(false);
  const [machineControlOpen, setMachineControlOpen] = useState(false);
  const [eomAuditorOpen, setEomAuditorOpen] = useState(false);
  const [triageOpen, setTriageOpen] = useState(false);
  const [ntripOpen, setNtripOpen] = useState(false);
  // Sprint 10 — Mining field tools
  const [setoutOpen, setSetoutOpen] = useState(false);
  const [mineGridOpen, setMineGridOpen] = useState(false);
  const [tunnelProfileOpen, setTunnelProfileOpen] = useState(false);
  const [safetyReportOpen, setSafetyReportOpen] = useState(false);
  // Sprint 10 — Marine field tools
  const [tidalDatumOpen, setTidalDatumOpen] = useState(false);
  const [backscatterOpen, setBackscatterOpen] = useState(false);
  const [qcDashboardOpen, setQcDashboardOpen] = useState(false);
  const [mbesSurveyOpen, setMbesSurveyOpen] = useState(false);
  // Sprint 10 — Stockpile change detection
  const [stockpileChangeOpen, setStockpileChangeOpen] = useState(false);
  // Sprint 11 — Real-time field ops
  const [roverStreamOpen, setRoverStreamOpen] = useState(false);
  const [tideGaugeOpen, setTideGaugeOpen] = useState(false);
  const [currentProject, setCurrentProject] = useState<MetarduProject | null>(null);
  const [layout, setLayout] = useState<LayoutProfile>(() => {
    // Initialize from persisted state
    if (typeof window !== "undefined") {
      return loadPersistedLayout();
    }
    return "default";
  });
  const [mapInstance, setMapInstance] = useState<Map | null>(null);
  const [profileActive, setProfileActive] = useState(false);
  const [csfResult, setCsfResult] = useState<CsfResult | null>(null);
  const [cubeSurface, setCubeSurface] = useState<CubeSurfaceRpc | null>(null);
  const [streamPings, setStreamPings] = useState<StreamPing[]>([]);
  const [isStreaming, setIsStreaming] = useState(false);
  const activeFileId = useSurveyStore((s) => s.activeFileId);

  const handleMapReady = useCallback((map: Map) => {
    setMapInstance(map);
  }, []);

  const { line: profileLine, clear: clearProfile } = useProfileTool({
    map: mapInstance,
    active: profileActive,
    domain: activeDomain,
  });

  // Ctrl+K opens command palette — case-insensitive so Caps Lock + Ctrl+K
  // and Shift+Ctrl+K both work. Tauri webview on Linux sometimes delivers
  // the uppercase variant depending on IME state.
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && (e.key === "k" || e.key === "K")) {
        e.preventDefault();
        setCommandPaletteOpen((v) => !v);
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  // Ctrl+Z / Ctrl+Y for undo/redo — Sprint 11. Skipped when the user is
  // typing in an input/textarea/select (so they can use the OS-native
  // undo in form fields).
  const undoFromStore = useUndoStore((s) => s.undo);
  const redoFromStore = useUndoStore((s) => s.redo);
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement | null;
      const tag = target?.tagName?.toLowerCase();
      const isEditable = tag === "input" || tag === "textarea" || tag === "select" || target?.isContentEditable;
      if (isEditable) return;
      if ((e.ctrlKey || e.metaKey) && !e.shiftKey && (e.key === "z" || e.key === "Z")) {
        e.preventDefault();
        undoFromStore();
      } else if ((e.ctrlKey || e.metaKey) && (e.key === "y" || e.key === "Y")) {
        e.preventDefault();
        redoFromStore();
      } else if ((e.ctrlKey || e.metaKey) && e.shiftKey && (e.key === "z" || e.key === "Z")) {
        // Ctrl+Shift+Z is also redo (common on Linux)
        e.preventDefault();
        redoFromStore();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [undoFromStore, redoFromStore]);

  // Project template → dialog opener (Sprint 11). Maps a DialogKey from
  // the template picker to the appropriate set<Dialog>Open(true) call.
  const handleOpenDialogs = useCallback((keys: DialogKey[]) => {
    for (const k of keys) {
      switch (k) {
        case "stockpileAudit": setStockpileAuditOpen(true); break;
        case "volumeCalc": setVolumeCalcOpen(true); break;
        case "stockpileChange": setStockpileChangeOpen(true); break;
        case "blastReport": setBlastReportOpen(true); break;
        case "highwall": setHighwallOpen(true); break;
        case "monitoring": setMonitoringOpen(true); break;
        case "eomAuditor": setEomAuditorOpen(true); break;
        case "eom": setEomOpen(true); break;
        case "csf": setCsfOpen(true); break;
        case "odm": setOdmOpen(true); break;
        case "machineControl": setMachineControlOpen(true); break;
        case "setout": setSetoutOpen(true); break;
        case "mineGrid": setMineGridOpen(true); break;
        case "tunnelProfile": setTunnelProfileOpen(true); break;
        case "safetyReport": setSafetyReportOpen(true); break;
        case "dredgeAudit": setDredgeAuditOpen(true); break;
        case "crossSection": setCrossSectionOpen(true); break;
        case "s44": setS44Open(true); break;
        case "s44Cert": setS44CertOpen(true); break;
        case "s57": setS57Open(true); break;
        case "cube": setCubeOpen(true); break;
        case "cubeDisambig": setCubeDisambigOpen(true); break;
        case "svp": setSvpOpen(true); break;
        case "vesselConfig": setVesselConfigOpen(true); break;
        case "sss": setSssOpen(true); break;
        case "densityGates": setDensityGatesOpen(true); break;
        case "tidalSpline": setTidalSplineOpen(true); break;
        case "tidalDatum": setTidalDatumOpen(true); break;
        case "backscatter": setBackscatterOpen(true); break;
        case "qcDashboard": setQcDashboardOpen(true); break;
        case "mbesSurvey": setMbesSurveyOpen(true); break;
        case "ml": setMlOpen(true); break;
        case "pipeline": setPipelineOpen(true); break;
        case "deliverable": setDeliverableOpen(true); break;
        case "sliceEditor": setSliceEditorOpen(true); break;
        case "benchmark": setBenchmarkOpen(true); break;
        case "ntrip": setNtripOpen(true); break;
        case "roverStream": setRoverStreamOpen(true); break;
        case "tideGauge": setTideGaugeOpen(true); break;
      }
    }
  }, []);

  // Apply layout profile settings when layout changes
  useEffect(() => {
    const settings = getLayoutSettings(layout);
    setSidebarOpen(settings.sidebarOpen);
    setRightPanelOpen(settings.rightPanelOpen);
  }, [layout]);

  // Responsive auto-collapse — when the viewport shrinks, panels give up
  // space so the map stays usable. We don't fight the user: if they
  // manually re-open a panel we let them, but layout switches and narrow
  // widths re-trigger this.
  useEffect(() => {
    if (viewport.isCompact) {
      // < lg: collapse the right panel so the map gets priority.
      setRightPanelOpen(false);
    }
    if (viewport.isVeryNarrow) {
      // < sm: collapse the inline sidebar; it becomes a drawer.
      setSidebarOpen(false);
    }
  }, [viewport.isCompact, viewport.isVeryNarrow]);

  // Auto-close drawer sidebar when the viewport grows past sm.
  useEffect(() => {
    if (!viewport.isVeryNarrow) {
      setDrawerSidebarOpen(false);
    }
  }, [viewport.isVeryNarrow]);

  // Lock body scroll when any dialog overlay is open. We piggy-back on the
  // CSS hook defined in index.css.
  useEffect(() => {
    const anyDialogOpen =
      settingsOpen ||
      volumeCalcOpen ||
      odmOpen ||
      csfOpen ||
      s44Open ||
      cubeOpen ||
      s57Open ||
      monitoringOpen ||
      mlOpen ||
      pipelineOpen ||
      eomOpen ||
      s44CertOpen ||
      commandPaletteOpen ||
      svpOpen ||
      vesselConfigOpen ||
      cubeDisambigOpen ||
      dredgeAuditOpen ||
      stockpileAuditOpen ||
      blastReportOpen ||
      highwallOpen ||
      crossSectionOpen ||
      deliverableOpen ||
      sssOpen ||
      sliceEditorOpen ||
      licenseOpen ||
      benchmarkOpen ||
      telemetryOpen ||
      projectOpen ||
      updateOpen ||
      marketplaceOpen ||
      densityGatesOpen ||
      tidalSplineOpen ||
      machineControlOpen ||
      eomAuditorOpen ||
      triageOpen ||
      ntripOpen ||
      setoutOpen ||
      mineGridOpen ||
      tunnelProfileOpen ||
      safetyReportOpen ||
      tidalDatumOpen ||
      backscatterOpen ||
      qcDashboardOpen ||
      mbesSurveyOpen ||
      stockpileChangeOpen ||
      roverStreamOpen ||
      tideGaugeOpen;
    document.body.classList.toggle("has-open-dialog", anyDialogOpen);
    return () => document.body.classList.remove("has-open-dialog");
  }, [
    settingsOpen,
    volumeCalcOpen,
    odmOpen,
    csfOpen,
    s44Open,
    cubeOpen,
    s57Open,
    monitoringOpen,
    mlOpen,
    pipelineOpen,
    eomOpen,
    s44CertOpen,
    commandPaletteOpen,
    svpOpen,
    vesselConfigOpen,
    cubeDisambigOpen,
    dredgeAuditOpen,
    stockpileAuditOpen,
    blastReportOpen,
    highwallOpen,
    crossSectionOpen,
    deliverableOpen,
    sssOpen,
    sliceEditorOpen,
    licenseOpen,
    benchmarkOpen,
    telemetryOpen,
    projectOpen,
    updateOpen,
    marketplaceOpen,
    densityGatesOpen,
    tidalSplineOpen,
    machineControlOpen,
    eomAuditorOpen,
    triageOpen,
    ntripOpen,
    setoutOpen,
    mineGridOpen,
    tunnelProfileOpen,
    safetyReportOpen,
    tidalDatumOpen,
    backscatterOpen,
    qcDashboardOpen,
    mbesSurveyOpen,
    stockpileChangeOpen,
    roverStreamOpen,
    tideGaugeOpen,
  ]);

  // Command palette actions
  const commandActions = useMemo(() => createCommandActions({
    onOpenVolumeCalc: () => setVolumeCalcOpen(true),
    onOpenOdm: () => setOdmOpen(true),
    onOpenCsf: () => setCsfOpen(true),
    onOpenS44: () => setS44Open(true),
    onOpenCube: () => setCubeOpen(true),
    onOpenS57: () => setS57Open(true),
    onOpenMl: () => setMlOpen(true),
    onOpenMonitoring: () => setMonitoringOpen(true),
    onOpenPipeline: () => setPipelineOpen(true),
    onOpenSettings: () => setSettingsOpen(true),
    onToggleProfile: () => setProfileActive((v) => !v),
    onToggleStream: () => setIsStreaming((v) => !v),
    onOpenEom: () => setEomOpen(true),
    onOpenS44Cert: () => setS44CertOpen(true),
    onOpenSvp: () => setSvpOpen(true),
    onOpenVesselConfig: () => setVesselConfigOpen(true),
    onOpenCubeDisambig: () => setCubeDisambigOpen(true),
    onOpenDredgeAudit: () => setDredgeAuditOpen(true),
    onOpenStockpileAudit: () => setStockpileAuditOpen(true),
    onOpenBlastReport: () => setBlastReportOpen(true),
    onOpenHighwall: () => setHighwallOpen(true),
    onOpenCrossSection: () => setCrossSectionOpen(true),
    onOpenDeliverable: () => setDeliverableOpen(true),
    onOpenSss: () => setSssOpen(true),
    onOpenSliceEditor: () => setSliceEditorOpen(true),
    onOpenLicense: () => setLicenseOpen(true),
    onOpenBenchmark: () => setBenchmarkOpen(true),
    onOpenTelemetry: () => setTelemetryOpen(true),
    onOpenProject: () => setProjectOpen(true),
    onOpenUpdate: () => setUpdateOpen(true),
    onOpenMarketplace: () => setMarketplaceOpen(true),
    onOpenDensityGates: () => setDensityGatesOpen(true),
    onOpenTidalSpline: () => setTidalSplineOpen(true),
    onOpenMachineControl: () => setMachineControlOpen(true),
    onOpenEomAuditor: () => setEomAuditorOpen(true),
    onOpenTriage: () => setTriageOpen(true),
    onOpenNtrip: () => setNtripOpen(true),
    onOpenSetout: () => setSetoutOpen(true),
    onOpenMineGrid: () => setMineGridOpen(true),
    onOpenTunnelProfile: () => setTunnelProfileOpen(true),
    onOpenSafetyReport: () => setSafetyReportOpen(true),
    onOpenTidalDatum: () => setTidalDatumOpen(true),
    onOpenBackscatter: () => setBackscatterOpen(true),
    onOpenQcDashboard: () => setQcDashboardOpen(true),
    onOpenMbesSurvey: () => setMbesSurveyOpen(true),
    onOpenStockpileChange: () => setStockpileChangeOpen(true),
    onOpenRoverStream: () => setRoverStreamOpen(true),
    onOpenTideGauge: () => setTideGaugeOpen(true),
  }), []);

  // Start/stop UDP streaming listener when the Radio button is toggled
  useEffect(() => {
    if (isStreaming) {
      startStream({
        port: 4000,
        buffer_size: 1000,
        flush_interval_ms: 500,
        format: "json",
      }).catch((err: unknown) => {
        console.error("Failed to start stream:", err);
        setIsStreaming(false);
      });
    } else {
      stopStream().catch((err: unknown) => {
        console.error("Failed to stop stream:", err);
      });
      setStreamPings([]);
    }
  }, [isStreaming]);

  // Sidebar JSX differs by viewport:
  //   - wide (≥ sm): rendered inline alongside the map when `sidebarOpen` is true.
  //   - very narrow (< sm): rendered as an overlay drawer when `drawerSidebarOpen`
  //     is true. The inline `sidebarOpen` flag stays false so it doesn't take space.
  //   - narrow (md range): rendered inline but in icon-only rail mode.
  const isInlineSidebar = sidebarOpen && !viewport.isVeryNarrow;
  const isDrawerSidebar = viewport.isVeryNarrow && drawerSidebarOpen;

  const sidebarActions = {
    onOpenSettings: () => setSettingsOpen(true),
    onOpenVolumeCalc: () => setVolumeCalcOpen(true),
    onOpenOdm: () => setOdmOpen(true),
    onOpenCsf: () => setCsfOpen(true),
    onOpenS44: () => setS44Open(true),
    onOpenCube: () => setCubeOpen(true),
    onOpenS57: () => setS57Open(true),
    onOpenMonitoring: () => setMonitoringOpen(true),
    onOpenMl: () => setMlOpen(true),
    onOpenPipeline: () => setPipelineOpen(true),
    onOpenSvp: () => setSvpOpen(true),
    onOpenVesselConfig: () => setVesselConfigOpen(true),
    onOpenCubeDisambig: () => setCubeDisambigOpen(true),
    onOpenDredgeAudit: () => setDredgeAuditOpen(true),
    onOpenStockpileAudit: () => setStockpileAuditOpen(true),
    onOpenBlastReport: () => setBlastReportOpen(true),
    onOpenHighwall: () => setHighwallOpen(true),
    onOpenCrossSection: () => setCrossSectionOpen(true),
    onOpenDeliverable: () => setDeliverableOpen(true),
    onOpenSss: () => setSssOpen(true),
    onOpenSliceEditor: () => setSliceEditorOpen(true),
    onOpenLicense: () => setLicenseOpen(true),
    onOpenBenchmark: () => setBenchmarkOpen(true),
    onOpenTelemetry: () => setTelemetryOpen(true),
    onOpenProject: () => setProjectOpen(true),
    onOpenUpdate: () => setUpdateOpen(true),
    onOpenMarketplace: () => setMarketplaceOpen(true),
    onOpenDensityGates: () => setDensityGatesOpen(true),
    onOpenTidalSpline: () => setTidalSplineOpen(true),
    onOpenMachineControl: () => setMachineControlOpen(true),
    onOpenEomAuditor: () => setEomAuditorOpen(true),
    onOpenTriage: () => setTriageOpen(true),
    onOpenNtrip: () => setNtripOpen(true),
    onOpenSetout: () => setSetoutOpen(true),
    onOpenMineGrid: () => setMineGridOpen(true),
    onOpenTunnelProfile: () => setTunnelProfileOpen(true),
    onOpenSafetyReport: () => setSafetyReportOpen(true),
    onOpenTidalDatum: () => setTidalDatumOpen(true),
    onOpenBackscatter: () => setBackscatterOpen(true),
    onOpenQcDashboard: () => setQcDashboardOpen(true),
    onOpenMbesSurvey: () => setMbesSurveyOpen(true),
    onOpenStockpileChange: () => setStockpileChangeOpen(true),
    onOpenRoverStream: () => setRoverStreamOpen(true),
    onOpenTideGauge: () => setTideGaugeOpen(true),
  };

  return (
    <div className="flex h-full w-full flex-col bg-navy-base">
      <TitleBar
        domain={activeDomain}
        layout={layout}
        onLayoutChange={setLayout}
        projectName={currentProject?.name ?? null}
        onToggleSidebar={() => {
          // Below sm we toggle the drawer; above we toggle the inline sidebar.
          if (viewport.isVeryNarrow) setDrawerSidebarOpen((v) => !v);
          else setSidebarOpen((v) => !v);
        }}
        onToggleRight={() => setRightPanelOpen((v) => !v)}
      />
      <div className="flex flex-1 overflow-hidden">
        {isInlineSidebar && (
          <LeftSidebar
            domain={activeDomain}
            railMode={viewport.isNarrow}
            {...sidebarActions}
          />
        )}

        {isDrawerSidebar && (
          <>
            <div
              className="drawer-backdrop absolute inset-0 z-40 lg:hidden"
              onClick={() => setDrawerSidebarOpen(false)}
              aria-hidden="true"
            />
            <div className="absolute left-0 top-0 bottom-0 z-50 w-[280px] max-w-[85vw]">
              <LeftSidebar
                domain={activeDomain}
                railMode={false}
                onDismiss={() => setDrawerSidebarOpen(false)}
                {...sidebarActions}
              />
            </div>
          </>
        )}

        <main
          className="relative flex-1 overflow-hidden"
          style={{ containerType: "inline-size", containerName: "map-canvas" }}
        >
          <MapCanvas
            domain={activeDomain}
            epsg={settings.defaultEpsg}
            onMapReady={handleMapReady}
          />
          <CubeSurfaceOverlay map={mapInstance} surface={cubeSurface} />
          <PointCloudLayer
            map={mapInstance}
            activeFileId={activeFileId}
            csfResult={csfResult}
            streamPings={streamPings}
          />
          <FileDropOverlay domain={activeDomain} />
          <CrsSwitchBanner />
          <FloatingActions
            onToggleSidebar={() => {
              if (viewport.isVeryNarrow) setDrawerSidebarOpen((v) => !v);
              else setSidebarOpen((v) => !v);
            }}
            onToggleRight={() => setRightPanelOpen((v) => !v)}
            onOpenSettings={() => setSettingsOpen(true)}
            onOpenVolumeCalc={() => setVolumeCalcOpen(true)}
            profileActive={profileActive}
            onToggleProfile={() => setProfileActive((v) => !v)}
            isStreaming={isStreaming}
            onToggleStream={() => {
              setIsStreaming((v) => !v);
              if (!isStreaming) setStreamPings([]);
            }}
          />
          <LiveStreamPanel
            isStreaming={isStreaming}
            onPings={(pings) => setStreamPings((prev) => [...prev.slice(-5000), ...pings])}
          />
          {profileActive && (
            <div
              className="pointer-events-none absolute left-1/2 top-12 z-30 -translate-x-1/2 rounded-md border px-3 py-1.5 text-[11px] backdrop-blur max-w-[90%]"
              style={{
                background: "rgba(10, 25, 47, 0.95)",
                borderColor: `${domainAccent[activeDomain].primary}60`,
                color: colors.steelLight,
              }}
            >
              {profileLine
                ? "Profile drawn — click again to redraw"
                : "Click two points on the map to draw a profile line"}
            </div>
          )}
        </main>
        {rightPanelOpen && (
          <RightPanel
            domain={activeDomain}
            epsg={settings.defaultEpsg}
            profileLine={profileLine}
            onClearProfile={clearProfile}
            profileActive={profileActive}
            onOpenCsf={() => setCsfOpen(true)}
            onOpenVolumeCalc={() => setVolumeCalcOpen(true)}
            onOpenEomAuditor={() => setEomAuditorOpen(true)}
            onOpenS44={() => setS44Open(true)}
            onOpenCube={() => setCubeOpen(true)}
            onOpenDeliverable={() => setDeliverableOpen(true)}
          />
        )}
      </div>
      <StatusBar domain={activeDomain} epsg={settings.defaultEpsg} map={mapInstance} />
      {/* ── Lazy-loaded dialogs ──
          Wrapped in a single <Suspense> with fallback={null}. Each
          dialog only renders content when `open` is true, so the
          fallback is only visible for the ~50-100ms it takes to load
          a dialog's chunk on first open — imperceptible to the user. */}
      <Suspense fallback={null}>
      <SettingsDialog open={settingsOpen} onClose={() => setSettingsOpen(false)} />
      <VolumeCalcDialog open={volumeCalcOpen} onClose={() => setVolumeCalcOpen(false)} />
      <OdmPipelineDialog open={odmOpen} onClose={() => setOdmOpen(false)} />
      <CsfClassificationDialog
        open={csfOpen}
        onClose={() => setCsfOpen(false)}
        onClassified={setCsfResult}
      />
      <S44ComplianceDialog open={s44Open} onClose={() => setS44Open(false)} />
      <CubeSurfaceDialog
        open={cubeOpen}
        onClose={() => setCubeOpen(false)}
        onSurfaceGenerated={setCubeSurface}
      />
      <S57ExportDialog open={s57Open} onClose={() => setS57Open(false)} />
      <Monitoring4DDialog open={monitoringOpen} onClose={() => setMonitoringOpen(false)} />
      <MlClassificationDialog open={mlOpen} onClose={() => setMlOpen(false)} />
      <PipelineEditorDialog open={pipelineOpen} onClose={() => setPipelineOpen(false)} />
      <EomReconciliationWizard open={eomOpen} onClose={() => setEomOpen(false)} />
      <S44CertificateDialog open={s44CertOpen} onClose={() => setS44CertOpen(false)} />
      <CommandPalette
        open={commandPaletteOpen}
        onClose={() => setCommandPaletteOpen(false)}
        actions={commandActions}
      />
      <SvpEditorDialog open={svpOpen} onClose={() => setSvpOpen(false)} />
      <VesselConfigDialog open={vesselConfigOpen} onClose={() => setVesselConfigOpen(false)} />
      <CubeDisambiguationDialog
        open={cubeDisambigOpen}
        onClose={() => setCubeDisambigOpen(false)}
        surface={cubeSurface}
      />
      <DredgeAuditWizard open={dredgeAuditOpen} onClose={() => setDredgeAuditOpen(false)} />
      <StockpileAuditWizard open={stockpileAuditOpen} onClose={() => setStockpileAuditOpen(false)} />
      <BlastReportWizard open={blastReportOpen} onClose={() => setBlastReportOpen(false)} />
      <HighwallMonitoringWizard open={highwallOpen} onClose={() => setHighwallOpen(false)} />
      <CrossSectionProfilerWizard open={crossSectionOpen} onClose={() => setCrossSectionOpen(false)} />
      <DeliverablePackageWizard open={deliverableOpen} onClose={() => setDeliverableOpen(false)} />
      <SssWaterfallViewer open={sssOpen} onClose={() => setSssOpen(false)} />
      <SliceEditor3D open={sliceEditorOpen} onClose={() => setSliceEditorOpen(false)} />
      <LicenseManagerDialog open={licenseOpen} onClose={() => setLicenseOpen(false)} />
      <BenchmarkDialog open={benchmarkOpen} onClose={() => setBenchmarkOpen(false)} />
      <TelemetryDialog open={telemetryOpen} onClose={() => setTelemetryOpen(false)} />
      <ProjectManagerDialog open={projectOpen} onClose={() => setProjectOpen(false)} currentProject={currentProject} onProjectLoaded={(p) => setCurrentProject(p)} onOpenDialogs={handleOpenDialogs} />
      <UpdateCheckerDialog open={updateOpen} onClose={() => setUpdateOpen(false)} />
      <PluginMarketplaceDialog open={marketplaceOpen} onClose={() => setMarketplaceOpen(false)} />
      <DensityGatesTool open={densityGatesOpen} onClose={() => setDensityGatesOpen(false)} />
      <TidalSplineTool open={tidalSplineOpen} onClose={() => setTidalSplineOpen(false)} />
      <MachineControlTool open={machineControlOpen} onClose={() => setMachineControlOpen(false)} />
      <EomAuditorDialog open={eomAuditorOpen} onClose={() => setEomAuditorOpen(false)} />
      <TriageDialog open={triageOpen} onClose={() => setTriageOpen(false)} />
      <NtripDialog open={ntripOpen} onClose={() => setNtripOpen(false)} />
      {/* ── Sprint 10: Mining field tools ── */}
      <SetoutToolDialog open={setoutOpen} onClose={() => setSetoutOpen(false)} />
      <MineGridDialog open={mineGridOpen} onClose={() => setMineGridOpen(false)} />
      <TunnelProfileDialog open={tunnelProfileOpen} onClose={() => setTunnelProfileOpen(false)} />
      <SafetyReportDialog open={safetyReportOpen} onClose={() => setSafetyReportOpen(false)} />
      {/* ── Sprint 10: Marine field tools ── */}
      <TidalDatumDialog open={tidalDatumOpen} onClose={() => setTidalDatumOpen(false)} />
      <BackscatterMosaicDialog open={backscatterOpen} onClose={() => setBackscatterOpen(false)} />
      <QcDashboardDialog open={qcDashboardOpen} onClose={() => setQcDashboardOpen(false)} />
      <MbesSurveyDialog
        open={mbesSurveyOpen}
        onClose={() => setMbesSurveyOpen(false)}
        onOpenQc={() => setQcDashboardOpen(true)}
        onOpenBackscatter={() => setBackscatterOpen(true)}
      />
      {/* ── Sprint 10: Stockpile change detection ── */}
      <StockpileChangeDialog open={stockpileChangeOpen} onClose={() => setStockpileChangeOpen(false)} />
      {/* ── Sprint 11: Real-time field ops ── */}
      <RoverStreamDialog open={roverStreamOpen} onClose={() => setRoverStreamOpen(false)} />
      <TideGaugeDialog open={tideGaugeOpen} onClose={() => setTideGaugeOpen(false)} />
      </Suspense>
    </div>
  );
}

/* ──────────────────────────────────────────────────────────── */

function TitleBar({
  domain,
  layout,
  onLayoutChange,
  onToggleSidebar,
  onToggleRight,
  projectName,
}: {
  domain: DomainMode;
  layout: LayoutProfile;
  onLayoutChange: (l: LayoutProfile) => void;
  onToggleSidebar: () => void;
  onToggleRight: () => void;
  projectName: string | null;
}) {
  const accent = domainAccent[domain].primary;
  // In browser mode the fake window controls (minimize/maximize/close) do
  // nothing — they're cosmetic leftovers from the Tauri shell. Hide them so
  // users don't click expecting the app to minimize.
  const showWindowControls = isNative();
  return (
    <header className="title-bar flex items-center justify-between gap-2 border-b border-navy-border bg-navy-panel px-2 sm:px-3">
      <div className="flex items-center gap-2 sm:gap-3 min-w-0">
        <button
          onClick={onToggleSidebar}
          title="Toggle sidebar"
          aria-label="Toggle sidebar"
          className="rounded p-1 text-steel-light hover:bg-navy-elevated hover:text-white flex-shrink-0"
        >
          <PanelLeft className="h-4 w-4" />
        </button>
        <BrandLogoMark size={28} className="flex-shrink-0" />
        <span className="text-[13px] font-semibold tracking-wide text-white hidden sm:inline truncate">
          Meta<span style={{ color: colors.industrialOrange }}>RDU</span> Industrial
        </span>
        <span className="text-steel-gray hidden md:inline">/</span>
        <span className="text-[13px] text-steel-light hidden md:inline truncate">
          {projectName ?? "No project loaded"}
        </span>
        <span
          className="rounded-sm px-1.5 py-0.5 text-[9px] font-semibold uppercase tracking-wider flex-shrink-0"
          style={{
            background: `${accent}20`,
            color: accent,
            border: `1px solid ${accent}40`,
          }}
        >
          {domainAccent[domain].label}
        </span>
      </div>

      <div className="flex items-center gap-2 sm:gap-3">
        <div className="hidden md:block">
          <LayoutProfiles active={layout} onChange={onLayoutChange} />
        </div>
        <button
          onClick={onToggleRight}
          title="Toggle right panel"
          aria-label="Toggle right panel"
          className="rounded p-1 text-steel-light hover:bg-navy-elevated hover:text-white"
        >
          <PanelRight className="h-4 w-4" />
        </button>
        {showWindowControls && (
          <div className="flex items-center gap-1">
            <button
              className="rounded p-1 text-steel-gray hover:bg-navy-elevated hover:text-white"
              title="Minimize"
              aria-label="Minimize window"
            >
              <Minus className="h-3.5 w-3.5" />
            </button>
            <button
              className="rounded p-1 text-steel-gray hover:bg-navy-elevated hover:text-white"
              title="Maximize"
              aria-label="Maximize window"
            >
              <Square className="h-3 w-3" />
            </button>
            <button
              className="rounded p-1 text-steel-gray hover:bg-fail/20 hover:text-fail"
              title="Close"
              aria-label="Close window"
            >
              <X className="h-3.5 w-3.5" />
            </button>
          </div>
        )}
      </div>
    </header>
  );
}

/* ──────────────────────────────────────────────────────────── */

function LeftSidebar({
  domain,
  railMode = false,
  onDismiss,
  onOpenSettings,
  onOpenVolumeCalc,
  onOpenOdm,
  onOpenCsf,
  onOpenS44,
  onOpenCube,
  onOpenS57,
  onOpenMonitoring,
  onOpenMl,
  onOpenPipeline,
  onOpenSvp,
  onOpenVesselConfig,
  onOpenCubeDisambig,
  onOpenDredgeAudit,
  onOpenStockpileAudit,
  onOpenBlastReport,
  onOpenHighwall,
  onOpenCrossSection,
  onOpenDeliverable,
  onOpenSss,
  onOpenSliceEditor,
  onOpenLicense,
  onOpenBenchmark,
  onOpenTelemetry,
  onOpenProject,
  onOpenUpdate,
  onOpenMarketplace,
  onOpenDensityGates,
  onOpenTidalSpline,
  onOpenMachineControl,
  onOpenEomAuditor,
  onOpenTriage,
  onOpenNtrip,
  onOpenSetout,
  onOpenMineGrid,
  onOpenTunnelProfile,
  onOpenSafetyReport,
  onOpenTidalDatum,
  onOpenBackscatter,
  onOpenQcDashboard,
  onOpenMbesSurvey,
  onOpenStockpileChange,
  onOpenRoverStream,
  onOpenTideGauge,
}: {
  domain: DomainMode;
  /** When true, sidebar collapses to icon-only rail (md-range widths). */
  railMode?: boolean;
  /** When set, renders a dismiss (×) button — used in drawer mode. */
  onDismiss?: () => void;
  onOpenSettings: () => void;
  onOpenVolumeCalc: () => void;
  onOpenOdm: () => void;
  onOpenCsf: () => void;
  onOpenS44: () => void;
  onOpenCube: () => void;
  onOpenS57: () => void;
  onOpenMonitoring: () => void;
  onOpenMl: () => void;
  onOpenPipeline: () => void;
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
  onOpenSetout: () => void;
  onOpenMineGrid: () => void;
  onOpenTunnelProfile: () => void;
  onOpenSafetyReport: () => void;
  onOpenTidalDatum: () => void;
  onOpenBackscatter: () => void;
  onOpenQcDashboard: () => void;
  onOpenMbesSurvey: () => void;
  onOpenStockpileChange: () => void;
  onOpenRoverStream: () => void;
  onOpenTideGauge: () => void;
}) {
  const accent = domainAccent[domain].primary;

  return (
    <aside
      className={`sidebar-transition flex h-full flex-col border-r border-navy-border bg-navy-panel ${
        railMode ? "sidebar-rail w-14" : "w-[260px]"
      }`}
    >
      <div className="border-b border-navy-border p-3 flex items-center gap-2">
        {railMode ? (
          <button
            title="New Survey"
            aria-label="New Survey"
            onClick={onOpenProject}
            className="flex h-8 w-8 items-center justify-center rounded-md transition-colors flex-shrink-0"
            style={{ background: accent, color: colors.navyBase }}
          >
            <Plus className="h-4 w-4" />
          </button>
        ) : (
          <button
            onClick={onOpenProject}
            className="new-survey-btn flex flex-1 items-center justify-center gap-2 rounded-md py-2.5 transition-colors"
            style={{ background: accent, color: colors.navyBase }}
          >
            <Plus className="h-4 w-4" />
            New Survey
          </button>
        )}
        {onDismiss && (
          <button
            onClick={onDismiss}
            title="Close menu"
            aria-label="Close menu"
            className="rounded p-1 text-steel-gray hover:bg-navy-elevated hover:text-white"
          >
            <X className="h-4 w-4" />
          </button>
        )}
      </div>

      <div className="flex-1 overflow-y-auto p-2">
        {/* ── Project ── */}
        <SidebarSection title="Project" icon={<Folder className="h-3 w-3" />}>
          <SidebarItem
            icon={<FileBox className="h-3 w-3" />}
            label="Project Manager"
            onClick={onOpenProject}
          />
          <SidebarItem
            icon={<FolderOpen className="h-3 w-3" />}
            label="Mission Triage"
            onClick={onOpenTriage}
          />
        </SidebarSection>

        {/* ── Mining ── */}
        {domain !== "marine" && (
          <SidebarSection title="Mining" icon={<Activity className="h-3 w-3" />}>
            <SidebarItem
              icon={<Terminal className="h-3 w-3" />}
              label="ODM Photogrammetry"
              onClick={onOpenOdm}
            />
            <SidebarItem
              icon={<Layers3 className="h-3 w-3" />}
              label="Classify Ground (CSF)"
              onClick={onOpenCsf}
            />
            <SidebarItem
              icon={<Calculator className="h-3 w-3" />}
              label="Volume Calculator"
              onClick={onOpenVolumeCalc}
            />
            <SidebarItem
              icon={<ShieldCheck className="h-3 w-3" />}
              label="EOM Volumetric Auditor"
              onClick={onOpenEomAuditor}
            />
            <div className="my-1.5 border-t border-navy-border" />
            <SidebarItem
              icon={<Boxes className="h-3 w-3" />}
              label="Stockpile Audit"
              onClick={onOpenStockpileAudit}
            />
            <SidebarItem
              icon={<Bomb className="h-3 w-3" />}
              label="Blast Report"
              onClick={onOpenBlastReport}
            />
            <SidebarItem
              icon={<ShieldAlert className="h-3 w-3" />}
              label="Highwall Monitoring"
              onClick={onOpenHighwall}
            />
            <SidebarItem
              icon={<History className="h-3 w-3" />}
              label="4D Monitoring"
              onClick={onOpenMonitoring}
            />
            <div className="my-1.5 border-t border-navy-border" />
            <SidebarItem
              icon={<Cpu className="h-3 w-3" />}
              label="Machine Control Compiler"
              onClick={onOpenMachineControl}
            />
            <SidebarItem
              icon={<Brain className="h-3 w-3" />}
              label="ML Classification"
              onClick={onOpenMl}
            />
            <div className="my-1.5 border-t border-navy-border" />
            <SidebarItem
              icon={<Crosshair className="h-3 w-3" />}
              label="Setting Out & Markout"
              onClick={onOpenSetout}
            />
            <SidebarItem
              icon={<Grid3x3 className="h-3 w-3" />}
              label="Mine Grid Transform"
              onClick={onOpenMineGrid}
            />
            <SidebarItem
              icon={<SquareDashed className="h-3 w-3" />}
              label="Tunnel Profile Analyzer"
              onClick={onOpenTunnelProfile}
            />
            <SidebarItem
              icon={<ShieldAlert className="h-3 w-3" />}
              label="Safety Inspection Report"
              onClick={onOpenSafetyReport}
            />
            <SidebarItem
              icon={<History className="h-3 w-3" />}
              label="Stockpile Change Detection"
              onClick={onOpenStockpileChange}
            />
          </SidebarSection>
        )}

        {/* ── Marine ── */}
        {domain !== "mining" && (
          <SidebarSection title="Marine" icon={<Waves className="h-3 w-3" />}>
            <SidebarItem
              icon={<Waves className="h-3 w-3" />}
              label="CUBE Surface"
              onClick={onOpenCube}
            />
            <SidebarItem
              icon={<Layers3 className="h-3 w-3" />}
              label="CUBE Disambiguation"
              onClick={onOpenCubeDisambig}
            />
            <SidebarItem
              icon={<Shield className="h-3 w-3" />}
              label="S-44 Compliance"
              onClick={onOpenS44}
            />
            <SidebarItem
              icon={<Anchor className="h-3 w-3" />}
              label="S-57 Export"
              onClick={onOpenS57}
            />
            <div className="my-1.5 border-t border-navy-border" />
            <SidebarItem
              icon={<Waves className="h-3 w-3" />}
              label="SVP Editor"
              onClick={onOpenSvp}
            />
            <SidebarItem
              icon={<Anchor className="h-3 w-3" />}
              label="Vessel Configuration"
              onClick={onOpenVesselConfig}
            />
            <SidebarItem
              icon={<Waves className="h-3 w-3" />}
              label="SSS Waterfall"
              onClick={onOpenSss}
            />
            <div className="my-1.5 border-t border-navy-border" />
            <SidebarItem
              icon={<Waves className="h-3 w-3" />}
              label="Dredge Audit"
              onClick={onOpenDredgeAudit}
            />
            <SidebarItem
              icon={<Ruler className="h-3 w-3" />}
              label="Cross-Section Profiler"
              onClick={onOpenCrossSection}
            />
            <SidebarItem
              icon={<Activity className="h-3 w-3" />}
              label="Density Gates"
              onClick={onOpenDensityGates}
            />
            <SidebarItem
              icon={<Waves className="h-3 w-3" />}
              label="Tidal Spline Corrector"
              onClick={onOpenTidalSpline}
            />
            <SidebarItem
              icon={<Brain className="h-3 w-3" />}
              label="ML Classification"
              onClick={onOpenMl}
            />
            <div className="my-1.5 border-t border-navy-border" />
            <SidebarItem
              icon={<FileSearch className="h-3 w-3" />}
              label="MBES Survey Reader (.all)"
              onClick={onOpenMbesSurvey}
            />
            <SidebarItem
              icon={<Activity className="h-3 w-3" />}
              label="Real-Time QC Dashboard"
              onClick={onOpenQcDashboard}
            />
            <SidebarItem
              icon={<Grid3x3 className="h-3 w-3" />}
              label="Backscatter Mosaic"
              onClick={onOpenBackscatter}
            />
            <SidebarItem
              icon={<Waves className="h-3 w-3" />}
              label="Tidal Datum Converter"
              onClick={onOpenTidalDatum}
            />
            <SidebarItem
              icon={<Waves className="h-3 w-3" />}
              label="Tide Gauge (NOAA / TCP)"
              onClick={onOpenTideGauge}
            />
          </SidebarSection>
        )}

        {/* ── Deliverables ── */}
        <SidebarSection title="Deliverables" icon={<Package className="h-3 w-3" />}>
          <SidebarItem
            icon={<Package className="h-3 w-3" />}
            label="Deliverable Package"
            onClick={onOpenDeliverable}
          />
          <SidebarItem
            icon={<Scissors className="h-3 w-3" />}
            label="3D Slice Editor"
            onClick={onOpenSliceEditor}
          />
        </SidebarSection>

        {/* ── Automation ── */}
        <SidebarSection title="Automation" icon={<GitBranch className="h-3 w-3" />}>
          <SidebarItem
            icon={<GitBranch className="h-3 w-3" />}
            label="Pipeline Editor"
            onClick={onOpenPipeline}
          />
        </SidebarSection>

        {/* ── Enterprise ── */}
        <SidebarSection title="Enterprise" icon={<Shield className="h-3 w-3" />}>
          <SidebarItem
            icon={<Radio className="h-3 w-3" />}
            label="NTRIP Client"
            onClick={onOpenNtrip}
          />
          <SidebarItem
            icon={<Satellite className="h-3 w-3" />}
            label="RTK Rover Stream"
            onClick={onOpenRoverStream}
          />
          <SidebarItem
            icon={<Key className="h-3 w-3" />}
            label="License Manager"
            onClick={onOpenLicense}
          />
          <SidebarItem
            icon={<Gauge className="h-3 w-3" />}
            label="Performance Benchmark"
            onClick={onOpenBenchmark}
          />
          <SidebarItem
            icon={<Activity className="h-3 w-3" />}
            label="Telemetry & Crash"
            onClick={onOpenTelemetry}
          />
          <SidebarItem
            icon={<Package className="h-3 w-3" />}
            label="Plugin Marketplace"
            onClick={onOpenMarketplace}
          />
          <SidebarItem
            icon={<RefreshCw className="h-3 w-3" />}
            label="Check for Updates"
            onClick={onOpenUpdate}
          />
        </SidebarSection>
      </div>

      <div className="border-t border-navy-border p-2">
        <SidebarItem
          icon={<Settings className="h-3 w-3" />}
          label="Settings"
          onClick={onOpenSettings}
        />
        <SidebarItem icon={<HelpCircle className="h-3 w-3" />} label="Help & Docs" onClick={onOpenSettings} />
      </div>
    </aside>
  );
}

function SidebarSection({
  title,
  icon,
  children,
}: {
  title: string;
  icon: React.ReactNode;
  children: React.ReactNode;
}) {
  return (
    <div className="mb-3">
      <div className="sidebar-section-label flex items-center gap-1.5 px-2 py-1.5 text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
        {icon}
        {title}
      </div>
      <div className="space-y-0.5">{children}</div>
    </div>
  );
}

function SidebarItem({
  icon,
  label,
  active = false,
  indent = false,
  onClick,
}: {
  icon?: React.ReactNode;
  label: string;
  active?: boolean;
  indent?: boolean;
  onClick?: () => void;
}) {
  return (
    <button
      onClick={onClick}
      title={label}
      className={`sidebar-item accent-bar-left focusable-row ${
        active ? "is-active" : ""
      } flex w-full items-center gap-2 rounded-md py-2 transition-colors ${
        active
          ? "bg-navy-elevated text-white"
          : "text-steel-light hover:bg-navy-elevated/50 hover:text-white"
      } ${indent ? "pl-6" : "pl-2"} pr-2`}
    >
      {icon && (
        <span className="sidebar-item-icon text-steel-gray flex-shrink-0">
          {icon}
        </span>
      )}
      <span className="flex-1 text-left truncate">{label}</span>
      {active && (
        <ChevronRight
          className="h-3 w-3 flex-shrink-0"
          style={{ color: colors.industrialOrange }}
        />
      )}
      {/* Tooltip — shown only in rail mode via CSS (see .sidebar-tooltip) */}
      <span className="sidebar-tooltip">{label}</span>
    </button>
  );
}

/* ──────────────────────────────────────────────────────────── */

function RightPanel({
  domain,
  epsg,
  profileLine,
  onClearProfile,
  profileActive,
  onOpenCsf,
  onOpenVolumeCalc,
  onOpenEomAuditor,
  onOpenS44,
  onOpenCube,
  onOpenDeliverable,
}: {
  domain: DomainMode;
  epsg: string;
  profileLine: ProfileLine | null;
  onClearProfile: () => void;
  profileActive: boolean;
  onOpenCsf: () => void;
  onOpenVolumeCalc: () => void;
  onOpenEomAuditor: () => void;
  onOpenS44: () => void;
  onOpenCube: () => void;
  onOpenDeliverable: () => void;
}) {
  const accent = domainAccent[domain].primary;
  const files = useSurveyStore((s) => s.files);
  const activeFileId = useSurveyStore((s) => s.activeFileId);
  const setActiveFile = useSurveyStore((s) => s.setActiveFile);
  const removeFile = useSurveyStore((s) => s.removeFile);

  const fileCount = files.length;
  const loadedCount = files.filter((f) => f.status === "loaded").length;
  const errorCount = files.filter((f) => f.status === "error").length;
  const probingCount = files.filter((f) => f.status === "probing").length;
  const totalSize = files.reduce((sum, f) => sum + f.size, 0);
  const totalPoints = files.reduce((sum, f) => sum + (f.pointCount ?? 0), 0);
  const lastAddedAt = files.reduce((latest, f) => Math.max(latest, f.addedAt), 0);
  const hasBounds = files.some((f) => f.bounds);
  const statusLabel =
    fileCount === 0
      ? "Ready for ingest"
      : errorCount > 0
        ? "Needs review"
        : probingCount > 0
          ? "Probing headers"
          : "Ready for processing";
  const nextActions =
    domain === "mining"
      ? [
          { title: "Classify ground", body: "Run CSF before volume calculations.", onClick: onOpenCsf },
          { title: "Compute volume", body: "Compare current DEM against a bench or previous survey.", onClick: onOpenVolumeCalc },
          { title: "EOM Auditor", body: "Automated LAS → signed PDF volume report.", onClick: onOpenEomAuditor },
        ]
      : domain === "marine"
        ? [
            { title: "Generate CUBE", body: "Build a defensible bathymetric surface.", onClick: onOpenCube },
            { title: "S-44 Compliance", body: "Run S-44 checks and verify TPU.", onClick: onOpenS44 },
            { title: "Package delivery", body: "Package S-57 outputs and audit manifest.", onClick: onOpenDeliverable },
          ]
        : [
            { title: "Ingest survey data", body: "Stage LAS/LAZ, GeoTIFF, MBES, drone, or control files.", onClick: () => {} },
            { title: "Run domain QC", body: "Use CSF/volumes for mining or density gates/CUBE for marine.", onClick: onOpenEomAuditor },
            { title: "Publish deliverables", body: "Package PDF reports, S-57 exports, and audit manifests.", onClick: onOpenDeliverable },
          ];
  const emptyTip =
    domain === "marine"
      ? "Drop a Kongsberg .all or Reson .s7k file anywhere to start the ingest pipeline."
      : domain === "mining"
        ? "Drop a LAS/LAZ point cloud or drone photogrammetry export to begin."
        : "Drop LAS/LAZ, GeoTIFF, MBES, side-scan, drone manifest, or control files to begin.";

  return (
    <aside className="sidebar-transition flex w-72 xl:w-80 flex-col border-l border-navy-border bg-navy-panel">
      <div className="border-b border-navy-border px-3 sm:px-4 py-3">
        <div className="flex items-center justify-between gap-2">
          <h3 className="text-xs font-semibold uppercase tracking-wider text-steel-light">
            Operations
          </h3>
          <span
            className="text-[10px] uppercase tracking-wider truncate"
            style={{ color: accent }}
          >
            {domainAccent[domain].label}
          </span>
        </div>
      </div>

      <div className="flex-1 overflow-y-auto p-4">
        {/* Status block */}
        <div className="rounded-lg border border-navy-border bg-navy-base p-4">
          <div className="flex items-center justify-between">
            <span className="text-xs text-steel-gray">{statusLabel}</span>
            <span
              className="rounded-sm px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wider"
              style={
                fileCount > 0
                  ? { background: `${colors.pass}20`, color: colors.pass }
                  : {
                      background: `${colors.investigate}20`,
                      color: colors.investigate,
                    }
              }
            >
              {fileCount > 0 ? `${loadedCount}/${fileCount} loaded` : "No data"}
            </span>
          </div>
          <div className="mt-3 text-2xl font-bold text-white">
            {fileCount > 0 ? fileCount : "—"}
          </div>
          <div className="text-xs text-steel-gray">
            staged files
          </div>
        </div>

        <div className="mt-3 grid grid-cols-3 gap-2">
          <MetricTile label="Points" value={totalPoints ? compactNumber(totalPoints) : "—"} />
          <MetricTile label="Data" value={totalSize ? formatBytes(totalSize) : "—"} />
          <MetricTile label="Errors" value={String(errorCount)} tone={errorCount > 0 ? "fail" : "pass"} />
        </div>

        {/* Dropped files list */}
        {fileCount > 0 && (
          <div className="mt-4">
            <h4 className="mb-2 text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
              Staged Files
            </h4>
            <div className="space-y-1.5">
              {files.map((f) => (
                <div
                  key={f.id}
                  onClick={() => setActiveFile(f.id)}
                  className={`cursor-pointer rounded-md border px-2.5 py-2 transition-colors ${
                    activeFileId === f.id
                      ? "bg-navy-elevated"
                      : "border-navy-border bg-navy-base hover:bg-navy-elevated/50"
                  }`}
                  style={
                    activeFileId === f.id
                      ? { borderColor: `${accent}80` }
                      : undefined
                  }
                >
                  <div className="flex items-center justify-between gap-2">
                    <span className="truncate text-xs font-medium text-white">
                      {f.name}
                    </span>
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        removeFile(f.id);
                      }}
                      className="text-steel-gray hover:text-fail"
                    >
                      <X className="h-3 w-3" />
                    </button>
                  </div>
                  <div className="mt-0.5 flex items-center justify-between text-[10px] text-steel-gray">
                    <span className="font-mono uppercase">{f.kind}</span>
                    <span>{formatBytes(f.size)}</span>
                  </div>
                  {f.status === "error" && f.errorMessage && (
                    <div
                      className="mt-1 text-[10px]"
                      style={{ color: colors.fail }}
                    >
                      {f.errorMessage}
                    </div>
                  )}
                </div>
              ))}
            </div>
          </div>
        )}

        {/* Quick stats */}
        <div className="mt-4 grid grid-cols-2 gap-2">
          <StatTile label="CRS" value={epsg} mono />
          <StatTile label="Domain" value={domainAccent[domain].label} />
          <StatTile
            label={domain === "marine" ? "S-44 Order" : "Coverage"}
            value={hasBounds ? "Bounds OK" : "Pending"}
          />
          <StatTile
            label="Last ingest"
            value={lastAddedAt ? new Date(lastAddedAt).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }) : "—"}
          />
        </div>

        <div className="mt-4">
          <h4 className="mb-2 text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
            Recommended next actions
          </h4>
          <div className="space-y-2">
            {nextActions.map((action) => (
              <button
                key={action.title}
                onClick={action.onClick}
                className="w-full rounded-md border border-navy-border bg-navy-base p-2.5 text-left transition-colors hover:border-industrial-orange/40 hover:bg-navy-elevated/50"
              >
                <div className="flex items-center justify-between">
                  <div className="text-xs font-semibold text-white">{action.title}</div>
                  <ChevronRight className="h-3 w-3 text-steel-gray" />
                </div>
                <div className="mt-0.5 text-[10px] leading-relaxed text-steel-gray">{action.body}</div>
              </button>
            ))}
          </div>
        </div>

        {/* Hint card */}
        <div
          className="mt-6 rounded-md border p-3 text-xs"
          style={{ borderColor: `${accent}40`, background: `${accent}10` }}
        >
          <div className="mb-1 font-semibold" style={{ color: accent }}>
            Tip
          </div>
          <p className="leading-relaxed text-steel-light">
            {fileCount === 0
              ? emptyTip
              : "Click a file to focus it on the map. Right-click for processing options."}
          </p>
        </div>
      </div>

      {/* Profile panel — slides in when profile tool is active */}
      {profileActive && (
        <div className="border-t border-navy-border">
          <div className="flex items-center justify-between border-b border-navy-border px-3 py-1.5">
            <span className="text-[10px] font-semibold uppercase tracking-wider text-steel-light">
              Profile
            </span>
            {profileLine && (
              <button
                onClick={onClearProfile}
                className="text-[10px] text-steel-gray hover:text-white"
              >
                Clear
              </button>
            )}
          </div>
          <div className="h-44">
            <ProfilePanel domain={domain} line={profileLine ? [profileLine.start, profileLine.end] : null} />
          </div>
        </div>
      )}
    </aside>
  );
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${(bytes / Math.pow(k, i)).toFixed(1)} ${sizes[i]}`;
}

function compactNumber(value: number): string {
  return Intl.NumberFormat(undefined, {
    notation: "compact",
    maximumFractionDigits: 1,
  }).format(value);
}

function MetricTile({
  label,
  value,
  tone = "neutral",
}: {
  label: string;
  value: string;
  tone?: "neutral" | "pass" | "fail";
}) {
  const color =
    tone === "pass" ? colors.pass : tone === "fail" ? colors.fail : colors.steelLight;
  return (
    <div className="rounded-md border border-navy-border bg-navy-base p-2">
      <div className="text-[9px] uppercase tracking-wider text-steel-gray">{label}</div>
      <div className="mt-0.5 truncate font-mono text-xs font-semibold" style={{ color }}>
        {value}
      </div>
    </div>
  );
}

function StatTile({
  label,
  value,
  mono = false,
}: {
  label: string;
  value: string;
  mono?: boolean;
}) {
  return (
    <div className="rounded-md border border-navy-border bg-navy-base p-2.5">
      <div className="text-[9px] uppercase tracking-wider text-steel-gray">
        {label}
      </div>
      <div
        className={`mt-0.5 truncate text-sm font-medium text-white ${
          mono ? "font-mono" : ""
        }`}
      >
        {value}
      </div>
    </div>
  );
}

/* ──────────────────────────────────────────────────────────── */

function FloatingActions({
  onToggleSidebar,
  onToggleRight,
  onOpenSettings,
  onOpenVolumeCalc,
  profileActive,
  onToggleProfile,
  isStreaming,
  onToggleStream,
}: {
  onToggleSidebar: () => void;
  onToggleRight: () => void;
  onOpenSettings: () => void;
  onOpenVolumeCalc: () => void;
  profileActive: boolean;
  onToggleProfile: () => void;
  isStreaming: boolean;
  onToggleStream: () => void;
}) {
  // Reusable button class — keeps the action buttons visually consistent.
  const baseBtn =
    "rounded border border-navy-border bg-navy-base/85 p-2 text-steel-light backdrop-blur transition-colors hover:bg-navy-elevated hover:text-white";
  return (
    <div className="absolute right-2 sm:right-3 top-2 sm:top-3 flex flex-col gap-1 z-20">
      <button
        onClick={onToggleRight}
        title="Toggle right panel"
        aria-label="Toggle right panel"
        className={baseBtn}
      >
        <Square className="h-3.5 w-3.5" />
      </button>
      <button
        onClick={onToggleSidebar}
        title="Toggle sidebar"
        aria-label="Toggle sidebar"
        className={baseBtn}
      >
        <Layers className="h-3.5 w-3.5" />
      </button>
      <button
        onClick={onToggleProfile}
        title="Profile tool"
        aria-label="Toggle profile tool"
        className="rounded border p-2 backdrop-blur transition-colors"
        style={{
          background: profileActive ? colors.industrialOrange : "rgba(10, 25, 47, 0.85)",
          borderColor: profileActive ? colors.industrialOrange : colors.navyBorder,
          color: profileActive ? colors.navyBase : colors.steelLight,
        }}
      >
        <TrendingUp className="h-3.5 w-3.5" />
      </button>
      <button
        onClick={onToggleStream}
        title="Live stream (UDP)"
        aria-label="Toggle live stream"
        className="rounded border p-2 backdrop-blur transition-colors"
        style={{
          background: isStreaming ? colors.marineTurquoise : "rgba(10, 25, 47, 0.85)",
          borderColor: isStreaming ? colors.marineTurquoise : colors.navyBorder,
          color: isStreaming ? colors.navyBase : colors.steelLight,
        }}
      >
        <Radio className={`h-3.5 w-3.5 ${isStreaming ? "animate-pulse" : ""}`} />
      </button>
      <button
        onClick={onOpenVolumeCalc}
        title="Volume calculator"
        aria-label="Open volume calculator"
        className={`hidden sm:block ${baseBtn}`}
      >
        <Calculator className="h-3.5 w-3.5" />
      </button>
      <button
        onClick={onOpenSettings}
        title="Settings"
        aria-label="Open settings"
        className={baseBtn}
      >
        <Settings className="h-3.5 w-3.5" />
      </button>
    </div>
  );
}

/* ──────────────────────────────────────────────────────────── */

function StatusBar({ domain, epsg, map }: { domain: DomainMode; epsg: string; map: Map | null }) {
  const accent = domainAccent[domain].primary;
  const [utcTime, setUtcTime] = useState(() => new Date().toISOString().slice(11, 19));
  const [coords, setCoords] = useState<string>("—");
  const [scale, setScale] = useState<string>("—");

  useEffect(() => {
    const timer = window.setInterval(() => {
      setUtcTime(new Date().toISOString().slice(11, 19));
    }, 1000);
    return () => window.clearInterval(timer);
  }, []);

  // Live cursor coordinates — the #1 feature that makes a GIS app feel professional
  useEffect(() => {
    if (!map) return;
    const onMouseMove = (e: { coordinate: number[] }) => {
      const [x, y] = e.coordinate;
      setCoords(`${x.toFixed(6)}, ${y.toFixed(6)}`);
      const res = map.getView().getResolution();
      if (res) {
        // Scale = resolution * 96 dpi / 0.0254 m/inch
        const scaleVal = Math.round(res * 96 / 0.0254);
        setScale(`1:${scaleVal.toLocaleString()}`);
      }
    };
    map.on("pointermove", onMouseMove);
    return () => { map.un("pointermove", onMouseMove); };
  }, [map]);

  // Undo stack indicator — Sprint 11
  const undoDepth = useUndoStore((s) => s.undoStack.length);
  const redoDepth = useUndoStore((s) => s.redoStack.length);
  const peekUndoDesc = useUndoStore((s) => s.undoStack.length > 0 ? s.undoStack[s.undoStack.length - 1].description : null);
  const undo = useUndoStore((s) => s.undo);
  const redo = useUndoStore((s) => s.redo);

  return (
    <footer className="status-bar">
      <div className="status-bar-item" style={{ color: colors.pass }}>
        <span className="h-1.5 w-1.5 rounded-full" style={{ background: colors.pass }} />
        Ready
      </div>
      <div className="status-bar-item">
        <Crosshair className="h-3 w-3" style={{ color: accent }} />
        <span style={{ color: colors.white }}>{coords}</span>
      </div>
      <div className="status-bar-item">
        <span style={{ color: colors.textMuted ?? colors.steelGray }}>Scale:</span>
        <span style={{ color: colors.steelLight }}>{scale}</span>
      </div>
      <div className="status-bar-item">
        <MapPin className="h-3 w-3" style={{ color: accent }} />
        <span style={{ color: colors.steelLight }}>{epsg}</span>
      </div>
      <div className="status-bar-item">
        <span style={{ color: accent }}>{domainAccent[domain].label}</span>
      </div>
      {/* Undo/Redo — Sprint 11 */}
      <div className="status-bar-item" title={peekUndoDesc ? `Undo: ${peekUndoDesc}` : "No actions to undo"}>
        <button
          onClick={() => undo()}
          disabled={undoDepth === 0}
          className="rounded px-1 py-0.5 text-[10px] disabled:opacity-30 hover:bg-navy-elevated"
          style={{ color: undoDepth > 0 ? colors.steelLight : colors.steelGray }}
          title={peekUndoDesc ? `Undo: ${peekUndoDesc} (Ctrl+Z)` : "Nothing to undo"}
        >
          ↶ Undo
        </button>
        <span className="text-[10px]" style={{ color: colors.steelGray }}>{undoDepth}</span>
        <button
          onClick={() => redo()}
          disabled={redoDepth === 0}
          className="rounded px-1 py-0.5 text-[10px] disabled:opacity-30 hover:bg-navy-elevated"
          style={{ color: redoDepth > 0 ? colors.steelLight : colors.steelGray }}
          title={`Redo (Ctrl+Y)`}
        >
          ↷ Redo
        </button>
        <span className="text-[10px]" style={{ color: colors.steelGray }}>{redoDepth}</span>
      </div>
      <div className="flex-1" />
      <div className="status-bar-item">
        <kbd className="rounded border px-1 py-0.5 font-mono text-[9px]" style={{ borderColor: colors.navyBorder, background: colors.navyBase }}>Ctrl+K</kbd>
        <span style={{ color: colors.steelGray }}>Commands</span>
      </div>
      <div className="status-bar-item">
        <Clock className="h-3 w-3" />
        <span className="tabular-nums">{utcTime}Z</span>
      </div>
      <div className="status-bar-item" style={{ color: colors.steelGray }}>
        v{APP_VERSION}
      </div>
    </footer>
  );
}
