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
  Settings,
  Minus,
  Square,
  X,
  Activity,
  Crosshair,
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
  Keyboard,
  Bookmark,
  SunMoon,
  Map as MapIcon,
  Eye,
  User,
  Search,
} from "lucide-react";
import { MapCanvas } from "@/components/map-canvas";
import { FileDropOverlay } from "@/components/file-drop-overlay";
import { CrsSwitchBanner } from "@/components/crs-switch-banner";
import { BrandLogoMark } from "@/components/brand-logo";
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
const ObnSurveyDialog = lazy(() => import("@/components/obn-survey-dialog").then(m => ({ default: m.ObnSurveyDialog })));
const PipelineInspectionDialog = lazy(() => import("@/components/pipeline-inspection-dialog").then(m => ({ default: m.PipelineInspectionDialog })));
// Sprint 10 — Stockpile change detection
const StockpileChangeDialog = lazy(() => import("@/components/stockpile-change-dialog").then(m => ({ default: m.StockpileChangeDialog })));
// Sprint 11 — Real-time field ops + QoL
const RoverStreamDialog = lazy(() => import("@/components/rover-stream-dialog").then(m => ({ default: m.RoverStreamDialog })));
const TideGaugeDialog = lazy(() => import("@/components/tide-gauge-dialog").then(m => ({ default: m.TideGaugeDialog })));
// Sprint 12 — UI polish
const KeyboardShortcutsHelp = lazy(() => import("@/components/keyboard-shortcuts-help").then(m => ({ default: m.KeyboardShortcutsHelp })));
// Sprint 13 — UI priorities
const SavedViewsDialog = lazy(() => import("@/components/saved-views-dialog").then(m => ({ default: m.SavedViewsDialog })));
const CustomizeToolbarDialog = lazy(() => import("@/components/customize-toolbar-dialog").then(m => ({ default: m.CustomizeToolbarDialog })));
// Sprint 16 — GIS gap feature dialogs
const IdwInterpolationDialog = lazy(() => import("@/components/idw-interpolation-dialog").then(m => ({ default: m.IdwInterpolationDialog })));
const ShapefileImportDialog = lazy(() => import("@/components/shapefile-import-dialog").then(m => ({ default: m.ShapefileImportDialog })));
const TopologyValidatorDialog = lazy(() => import("@/components/topology-validator-dialog").then(m => ({ default: m.TopologyValidatorDialog })));
// Sprint 17 — UI features + map layout
const MapLayoutDialog = lazy(() => import("@/components/map-layout-dialog").then(m => ({ default: m.MapLayoutDialog })));
// Sprint 20 — account management
const AccountDialog = lazy(() => import("@/components/account-dialog").then(m => ({ default: m.AccountDialog })));
import { useProfileTool } from "@/lib/use-profile-tool";
import type { CsfResult, CubeSurfaceRpc, MetarduProject } from "@/lib/tauri-ipc";
import { startStream, stopStream } from "@/lib/tauri-ipc";
import { useUndoStore } from "@/stores/undo-store";
import type { DialogKey } from "@/lib/project-templates";
import { MapOverlays, type MapLayer } from "@/components/map-overlays";
import { useKeyboardShortcutsHelp } from "@/components/keyboard-shortcuts-help";
import { CustomizableToolbar } from "@/components/customizable-toolbar";
import { useTheme } from "@/lib/theme-auto";
import { BasemapSwitcher } from "@/components/basemap-switcher";
import { OrthomosaicOverlay } from "@/components/orthomosaic-overlay";
import { useColorblindPalette } from "@/lib/colorblind-palette";
import { WorkflowRail, type WorkflowActions, type WorkflowStage } from "@/components/workflow-rail";
import { SurveyBriefPanel } from "@/components/survey-brief-panel";
import { WorkspaceStatusBar } from "@/components/workspace-status-bar";
import {
  colors,
  domainAccent,
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
  const [obnSurveyOpen, setObnSurveyOpen] = useState(false);
  const [pipelineInspectionOpen, setPipelineInspectionOpen] = useState(false);
  // Sprint 10 — Stockpile change detection
  const [stockpileChangeOpen, setStockpileChangeOpen] = useState(false);
  // Sprint 11 — Real-time field ops
  const [roverStreamOpen, setRoverStreamOpen] = useState(false);
  const [tideGaugeOpen, setTideGaugeOpen] = useState(false);
  // Sprint 12 — UI polish: keyboard shortcuts overlay + map layer toggle
  const shortcutsHelp = useKeyboardShortcutsHelp();
  const [mapLayers, setMapLayers] = useState<MapLayer[]>([
    { id: "pointcloud", label: "Point Cloud", visible: true, icon: "pointcloud" },
    { id: "dem", label: "DEM Hillshade", visible: true, icon: "dem" },
    { id: "cube", label: "CUBE Surface", visible: true, icon: "cube" },
    { id: "stream", label: "Live Stream", visible: false, icon: "stream" },
    { id: "orthomosaic", label: "Orthomosaic", visible: true, icon: "dem" },
  ]);
  const [mapRotation, setMapRotation] = useState(0);
  // Sprint 13 — saved views + customize toolbar + theme
  const [savedViewsOpen, setSavedViewsOpen] = useState(false);
  const [customizeToolbarOpen, setCustomizeToolbarOpen] = useState(false);
  const { mode: themeMode, setMode: setThemeMode } = useTheme();
  // Sprint 16 — GIS gap feature dialogs
  const [idwOpen, setIdwOpen] = useState(false);
  const [shapefileOpen, setShapefileOpen] = useState(false);
  const [topologyOpen, setTopologyOpen] = useState(false);
  // Sprint 17 — map layout + orthomosaic + colorblind
  const [mapLayoutOpen, setMapLayoutOpen] = useState(false);
  const [orthoPath, setOrthoPath] = useState<string | null>(null);
  const colorblind = useColorblindPalette();
  // Sprint 20 — account management
  const [accountOpen, setAccountOpen] = useState(false);
  const [workflowStage, setWorkflowStage] = useState<WorkflowStage>("ingest");

  // Sprint 18 — orthomosaic file picker (opens native dialog, triggers overlay)
  const handlePickOrthomosaic = useCallback(async () => {
    try {
      const { pickFile } = await import("@/lib/file-picker");
      const path = await pickFile({
        extensions: ["tif", "tiff"],
        filterName: "GeoTIFF Orthomosaic",
        title: "Select orthomosaic GeoTIFF",
      });
      if (path) {
        setOrthoPath(path);
      }
    } catch (err) {
      console.error("Orthomosaic file picker error:", err);
    }
  }, []);
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
    // Sprint 12 — track map rotation for the north arrow overlay
    const updateRotation = () => setMapRotation(map.getView().getRotation() || 0);
    map.on("moveend", updateRotation);
    updateRotation();
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
        case "obnSurvey": setObnSurveyOpen(true); break;
        case "pipelineInspection": setPipelineInspectionOpen(true); break;
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
      obnSurveyOpen ||
      pipelineInspectionOpen ||
      stockpileChangeOpen ||
      roverStreamOpen ||
      tideGaugeOpen ||
      idwOpen ||
      shapefileOpen ||
      topologyOpen ||
      accountOpen;
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
    obnSurveyOpen,
    pipelineInspectionOpen,
    stockpileChangeOpen,
    roverStreamOpen,
    tideGaugeOpen,
    idwOpen,
    shapefileOpen,
    topologyOpen,
    accountOpen,
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
    onOpenObnSurvey: () => setObnSurveyOpen(true),
    onOpenPipelineInspection: () => setPipelineInspectionOpen(true),
    onOpenStockpileChange: () => setStockpileChangeOpen(true),
    onOpenRoverStream: () => setRoverStreamOpen(true),
    onOpenTideGauge: () => setTideGaugeOpen(true),
    onOpenShortcuts: () => shortcutsHelp.setOpen(true),
    onOpenSavedViews: () => setSavedViewsOpen(true),
    onOpenCustomizeToolbar: () => setCustomizeToolbarOpen(true),
    onToggleTheme: () => setThemeMode(themeMode === "dark" ? "light" : "dark"),
    onOpenIdw: () => setIdwOpen(true),
    onOpenShapefile: () => setShapefileOpen(true),
    onOpenTopology: () => setTopologyOpen(true),
    onOpenMapLayout: () => setMapLayoutOpen(true),
    onToggleColorblind: () => colorblind.toggle(),
    onOpenAccount: () => setAccountOpen(true),
  }), [themeMode, setThemeMode, colorblind]);

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
    onOpenShortcuts: () => shortcutsHelp.setOpen(true),
    onOpenSavedViews: () => setSavedViewsOpen(true),
    onToggleTheme: () => setThemeMode(themeMode === "dark" ? "light" : "dark"),
    onOpenIdw: () => setIdwOpen(true),
    onOpenShapefile: () => setShapefileOpen(true),
    onOpenTopology: () => setTopologyOpen(true),
    onOpenMapLayout: () => setMapLayoutOpen(true),
    onToggleColorblind: () => colorblind.toggle(),
    onPickOrthomosaic: handlePickOrthomosaic,
    onOpenAccount: () => setAccountOpen(true),
    onOpenObnSurvey: () => setObnSurveyOpen(true),
    onOpenPipelineInspection: () => setPipelineInspectionOpen(true),
  };
  const workflowActions: WorkflowActions = {
    ingest: () => setProjectOpen(true),
    validate: () => activeDomain === "mining" ? setCsfOpen(true) : setS44Open(true),
    process: () => activeDomain === "mining" ? setVolumeCalcOpen(true) : setCubeOpen(true),
    review: () => activeDomain === "mining" ? setEomAuditorOpen(true) : setS44Open(true),
    deliver: () => setDeliverableOpen(true),
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
        onOpenCommands={() => setCommandPaletteOpen(true)}
      />
      <WorkflowRail
        domain={activeDomain}
        stage={workflowStage}
        onStageChange={setWorkflowStage}
        actions={workflowActions}
      />
      {/* Sprint 13 — customizable toolbar with pinned actions */}
      <CustomizableToolbar
        onOpenDialog={(key) => handleOpenDialogs([key as DialogKey])}
        onCustomize={() => setCustomizeToolbarOpen(true)}
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
          {/* Sprint 12 — north arrow + layer toggle */}
          <MapOverlays
            rotation={mapRotation}
            layers={mapLayers}
            onToggleLayer={(id) => setMapLayers((prev) => prev.map((l) => l.id === id ? { ...l, visible: !l.visible } : l))}
          />
          {/* Sprint 17 — orthomosaic overlay (georeferenced RGB) */}
          <OrthomosaicOverlay
            map={mapInstance}
            orthoPath={orthoPath}
            visible={mapLayers.find((l) => l.id === "orthomosaic")?.visible ?? true}
          />
          {/* Sprint 17 — Unified Top-Right Map HUD */}
          <div className="absolute right-3 top-3 z-10 flex items-center gap-1.5 p-1 rounded-md bg-[#111827]/90 backdrop-blur-md border border-[#263345] shadow-lg">
            <button
              type="button"
              onClick={() => setProfileActive((v) => !v)}
              className={`flex items-center gap-1.5 px-2.5 py-1 text-xs font-medium rounded transition-colors ${
                profileActive
                  ? "bg-amber-500/20 text-amber-300 border border-amber-500/40"
                  : "text-slate-300 hover:text-white hover:bg-slate-800/60"
              }`}
              title="Draw Profile Line (Click two points on canvas)"
            >
              <Activity className="w-3.5 h-3.5" />
              <span>Profile</span>
            </button>
            <button
              type="button"
              onClick={() => {
                setIsStreaming((v) => !v);
                if (!isStreaming) setStreamPings([]);
              }}
              className={`flex items-center gap-1.5 px-2.5 py-1 text-xs font-medium rounded transition-colors ${
                isStreaming
                  ? "bg-emerald-500/20 text-emerald-300 border border-emerald-500/40 animate-pulse"
                  : "text-slate-300 hover:text-white hover:bg-slate-800/60"
              }`}
              title="Toggle Live Bathymetry / Telemetry Stream (UDP)"
            >
              <Radio className="w-3.5 h-3.5" />
              <span>{isStreaming ? "Streaming" : "Live Stream"}</span>
            </button>
            <div className="h-4 w-px bg-[#263345]" />
            <BasemapSwitcher map={mapInstance} />
          </div>
          <CubeSurfaceOverlay map={mapInstance} surface={cubeSurface} />
          <PointCloudLayer
            map={mapInstance}
            activeFileId={activeFileId}
            csfResult={csfResult}
            streamPings={streamPings}
          />
          <FileDropOverlay domain={activeDomain} />
          <CrsSwitchBanner />
          {/* Top-center notification stack — Sprint 11 UI fix */}
          {profileActive && (
            <div
              className="pointer-events-none absolute left-1/2 top-20 z-20 -translate-x-1/2 rounded-md border px-3 py-1.5 text-[11px] backdrop-blur max-w-[90%]"
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
          <LiveStreamPanel
            isStreaming={isStreaming}
            onPings={(pings) => setStreamPings((prev) => [...prev.slice(-5000), ...pings])}
          />
        </main>
        {rightPanelOpen && (
          <SurveyBriefPanel
            domain={activeDomain}
            epsg={settings.defaultEpsg}
            profileLine={profileLine}
            onClearProfile={clearProfile}
            profileActive={profileActive}
            actions={{
              classify: () => setCsfOpen(true),
              volume: () => setVolumeCalcOpen(true),
              audit: () => setEomAuditorOpen(true),
              cube: () => setCubeOpen(true),
              s44: () => setS44Open(true),
              deliver: () => setDeliverableOpen(true),
            }}
          />
        )}
      </div>
      <WorkspaceStatusBar domain={activeDomain} epsg={settings.defaultEpsg} map={mapInstance} onOpenShortcuts={() => shortcutsHelp.setOpen(true)} />
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
      <ObnSurveyDialog open={obnSurveyOpen} onClose={() => setObnSurveyOpen(false)} />
      <PipelineInspectionDialog open={pipelineInspectionOpen} onClose={() => setPipelineInspectionOpen(false)} />
      {/* ── Sprint 10: Stockpile change detection ── */}
      <StockpileChangeDialog open={stockpileChangeOpen} onClose={() => setStockpileChangeOpen(false)} />
      {/* ── Sprint 11: Real-time field ops ── */}
      <RoverStreamDialog open={roverStreamOpen} onClose={() => setRoverStreamOpen(false)} />
      <TideGaugeDialog open={tideGaugeOpen} onClose={() => setTideGaugeOpen(false)} />
      {/* Sprint 12 — keyboard shortcuts overlay */}
      <KeyboardShortcutsHelp open={shortcutsHelp.open} onClose={() => shortcutsHelp.setOpen(false)} />
      {/* Sprint 13 — saved views + customize toolbar */}
      <SavedViewsDialog
        open={savedViewsOpen}
        onClose={() => setSavedViewsOpen(false)}
        onCapture={() => {
          if (!mapInstance) return null;
          const view = mapInstance.getView();
          const center = view.getCenter();
          if (!center) return null;
          return {
            center: [center[0], center[1]] as [number, number],
            zoom: view.getZoom() ?? 1,
            rotation: view.getRotation() ?? 0,
            layers: Object.fromEntries(mapLayers.map((l) => [l.id, l.visible])),
            domain: activeDomain,
            epsg: settings.defaultEpsg,
          };
        }}
        onRestore={(view) => {
          if (!mapInstance) return;
          const mapView = mapInstance.getView();
          mapView.setCenter(view.center);
          mapView.setZoom(view.zoom);
          mapView.setRotation(view.rotation);
          setMapLayers((prev) => prev.map((l) => ({ ...l, visible: view.layers[l.id] ?? l.visible })));
        }}
      />
      <CustomizeToolbarDialog open={customizeToolbarOpen} onClose={() => setCustomizeToolbarOpen(false)} />
      {/* Sprint 16 — GIS gap feature dialogs */}
      <IdwInterpolationDialog open={idwOpen} onClose={() => setIdwOpen(false)} />
      <ShapefileImportDialog open={shapefileOpen} onClose={() => setShapefileOpen(false)} />
      <TopologyValidatorDialog open={topologyOpen} onClose={() => setTopologyOpen(false)} />
      {/* Sprint 20 — account management */}
      <AccountDialog open={accountOpen} onClose={() => setAccountOpen(false)} />
      {/* Sprint 17 — map layout + orthomosaic */}
      <MapLayoutDialog
        open={mapLayoutOpen}
        onClose={() => setMapLayoutOpen(false)}
        map={mapInstance}
        defaultProjectName={currentProject?.name ?? ""}
        defaultCrs={settings.defaultEpsg}
      />
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
  onOpenCommands,
}: {
  domain: DomainMode;
  layout: LayoutProfile;
  onLayoutChange: (l: LayoutProfile) => void;
  onToggleSidebar: () => void;
  onToggleRight: () => void;
  projectName: string | null;
  onOpenCommands: () => void;
}) {
  const accent = domainAccent[domain].primary;
  const showWindowControls = isNative();
  return (
    <header className="title-bar title-bar-enterprise flex items-center justify-between gap-2 border-b border-[#263345] bg-[#111827]/95 px-3 backdrop-blur-md">
      <div className="flex items-center gap-2.5 min-w-0">
        <button
          onClick={onToggleSidebar}
          title="Toggle sidebar (Ctrl+B)"
          aria-label="Toggle sidebar"
          className="rounded p-1 text-slate-400 hover:bg-[#1F2937] hover:text-white transition-colors flex-shrink-0"
        >
          <PanelLeft className="h-4 w-4" />
        </button>
        <BrandLogoMark size={24} className="flex-shrink-0" />
        <div className="flex items-baseline gap-1 hidden sm:flex">
          <span className="text-[13px] font-semibold tracking-tight text-white">
            Meta<span className="text-[#F97316]">RDU</span>
          </span>
          <span className="text-[11px] font-medium text-slate-400">Industrial</span>
        </div>
        <span className="text-slate-600 hidden md:inline">/</span>
        <span className="text-[12px] font-medium text-slate-300 hidden md:inline truncate max-w-[200px]">
          {projectName ?? "Untitled Survey"}
        </span>
        <span
          className="rounded px-1.5 py-0.5 text-[9px] font-mono font-semibold uppercase tracking-wider flex-shrink-0 border"
          style={{
            background: `${accent}15`,
            color: accent,
            borderColor: `${accent}35`,
          }}
        >
          {domainAccent[domain].label}
        </span>
      </div>

      {/* Central Quick Search / Action launcher */}
      <div className="flex-1 max-w-sm mx-2 hidden sm:block">
        <button
          onClick={onOpenCommands}
          className="flex w-full items-center justify-between rounded-md border border-[#263345] bg-[#0B0F17]/80 px-2.5 py-1 text-xs text-slate-400 hover:border-slate-600 hover:text-slate-200 transition-colors shadow-inner"
        >
          <div className="flex items-center gap-2 truncate">
            <Search className="h-3.5 w-3.5 text-slate-500 flex-shrink-0" />
            <span className="truncate">Search tools, commands & files...</span>
          </div>
          <kbd className="rounded border border-[#374151] bg-[#1F2937] px-1.5 py-0.5 font-mono text-[9px] text-slate-400 flex-shrink-0 ml-2">
            Ctrl+K
          </kbd>
        </button>
      </div>

      <div className="flex items-center gap-2">
        <div className="hidden lg:block">
          <LayoutProfiles active={layout} onChange={onLayoutChange} />
        </div>
        <button
          onClick={onToggleRight}
          title="Toggle operations panel"
          aria-label="Toggle operations panel"
          className="rounded p-1 text-slate-400 hover:bg-[#1F2937] hover:text-white transition-colors"
        >
          <PanelRight className="h-4 w-4" />
        </button>
        {showWindowControls && (
          <div className="flex items-center gap-0.5 ml-1 border-l border-[#263345] pl-1.5">
            <button
              className="rounded p-1 text-slate-400 hover:bg-[#1F2937] hover:text-white transition-colors"
              title="Minimize"
              aria-label="Minimize window"
            >
              <Minus className="h-3 w-3" />
            </button>
            <button
              className="rounded p-1 text-slate-400 hover:bg-[#1F2937] hover:text-white transition-colors"
              title="Maximize"
              aria-label="Maximize window"
            >
              <Square className="h-2.5 w-2.5" />
            </button>
            <button
              className="rounded p-1 text-slate-400 hover:bg-rose-600 hover:text-white transition-colors"
              title="Close"
              aria-label="Close window"
            >
              <X className="h-3 w-3" />
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
  onOpenShortcuts,
  onOpenSavedViews,
  onToggleTheme,
  onOpenIdw,
  onOpenShapefile,
  onOpenTopology,
  onOpenMapLayout,
  onToggleColorblind,
  onPickOrthomosaic,
  onOpenAccount,
  onOpenObnSurvey,
  onOpenPipelineInspection,
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
  onOpenShortcuts: () => void;
  onOpenSavedViews: () => void;
  onToggleTheme: () => void;
  onOpenIdw: () => void;
  onOpenShapefile: () => void;
  onOpenTopology: () => void;
  onOpenMapLayout: () => void;
  onToggleColorblind: () => void;
  onPickOrthomosaic: () => void;
  onOpenAccount: () => void;
  onOpenObnSurvey: () => void;
  onOpenPipelineInspection: () => void;
}) {
  const [filterQuery, setFilterQuery] = useState("");

  const allTools = useMemo(() => [
    // Project
    { id: "proj-mgr", label: "Project Manager", category: "Project", icon: <FileBox className="h-3.5 w-3.5" />, onClick: onOpenProject },
    { id: "triage", label: "Mission Triage", category: "Project", icon: <FolderOpen className="h-3.5 w-3.5" />, onClick: onOpenTriage },
    // Marine & Subsea
    { id: "pipe-insp", label: "Pipeline Inspection (EIVA / AutoCAD)", category: "Marine & Subsea", icon: <GitBranch className="h-3.5 w-3.5" />, onClick: onOpenPipelineInspection, domain: "marine", badge: "DNV" },
    { id: "obn-survey", label: "OBN Seismic Survey (4DNav)", category: "Marine & Subsea", icon: <Radio className="h-3.5 w-3.5" />, onClick: onOpenObnSurvey, domain: "marine", badge: "P1/11" },
    { id: "cube", label: "CUBE Bathymetric Surface", category: "Marine & Subsea", icon: <Waves className="h-3.5 w-3.5" />, onClick: onOpenCube, domain: "marine" },
    { id: "cube-disambig", label: "CUBE Disambiguation", category: "Marine & Subsea", icon: <Layers3 className="h-3.5 w-3.5" />, onClick: onOpenCubeDisambig, domain: "marine" },
    { id: "s44", label: "S-44 TPU Compliance", category: "Marine & Subsea", icon: <Shield className="h-3.5 w-3.5" />, onClick: onOpenS44, domain: "marine" },
    { id: "s57", label: "S-57 Nautical Chart Export", category: "Marine & Subsea", icon: <Anchor className="h-3.5 w-3.5" />, onClick: onOpenS57, domain: "marine" },
    { id: "svp", label: "SVP Sound Velocity Editor", category: "Marine & Subsea", icon: <Waves className="h-3.5 w-3.5" />, onClick: onOpenSvp, domain: "marine" },
    { id: "vessel", label: "Vessel Configuration", category: "Marine & Subsea", icon: <Anchor className="h-3.5 w-3.5" />, onClick: onOpenVesselConfig, domain: "marine" },
    { id: "sss", label: "Side-Scan Sonar Waterfall", category: "Marine & Subsea", icon: <Waves className="h-3.5 w-3.5" />, onClick: onOpenSss, domain: "marine" },
    { id: "mbes", label: "MBES Survey Reader (.all)", category: "Marine & Subsea", icon: <FileSearch className="h-3.5 w-3.5" />, onClick: onOpenMbesSurvey, domain: "marine" },
    { id: "qc-dash", label: "Real-Time QC Dashboard", category: "Marine & Subsea", icon: <Activity className="h-3.5 w-3.5" />, onClick: onOpenQcDashboard, domain: "marine" },
    { id: "backscatter", label: "Backscatter Mosaic", category: "Marine & Subsea", icon: <Grid3x3 className="h-3.5 w-3.5" />, onClick: onOpenBackscatter, domain: "marine" },
    { id: "tidal-datum", label: "Tidal Datum Converter", category: "Marine & Subsea", icon: <Waves className="h-3.5 w-3.5" />, onClick: onOpenTidalDatum, domain: "marine" },
    { id: "tidal-spline", label: "Tidal Spline Corrector", category: "Marine & Subsea", icon: <Waves className="h-3.5 w-3.5" />, onClick: onOpenTidalSpline, domain: "marine" },
    { id: "tide-gauge", label: "Tide Gauge Monitor (NOAA / TCP)", category: "Marine & Subsea", icon: <Waves className="h-3.5 w-3.5" />, onClick: onOpenTideGauge, domain: "marine" },
    { id: "dredge", label: "Dredge Progress Audit", category: "Marine & Subsea", icon: <Waves className="h-3.5 w-3.5" />, onClick: onOpenDredgeAudit, domain: "marine" },
    // Mining Operations
    { id: "odm", label: "ODM Photogrammetry", category: "Mining Operations", icon: <Terminal className="h-3.5 w-3.5" />, onClick: onOpenOdm, domain: "mining" },
    { id: "csf", label: "Classify Ground (CSF)", category: "Mining Operations", icon: <Layers3 className="h-3.5 w-3.5" />, onClick: onOpenCsf, domain: "mining" },
    { id: "vol-calc", label: "Volume Calculator", category: "Mining Operations", icon: <Calculator className="h-3.5 w-3.5" />, onClick: onOpenVolumeCalc, domain: "mining" },
    { id: "eom", label: "EOM Volumetric Auditor", category: "Mining Operations", icon: <ShieldCheck className="h-3.5 w-3.5" />, onClick: onOpenEomAuditor, domain: "mining", badge: "Audit" },
    { id: "stockpile", label: "Stockpile Audit", category: "Mining Operations", icon: <Boxes className="h-3.5 w-3.5" />, onClick: onOpenStockpileAudit, domain: "mining" },
    { id: "stockpile-chg", label: "Stockpile Change Detection", category: "Mining Operations", icon: <History className="h-3.5 w-3.5" />, onClick: onOpenStockpileChange, domain: "mining" },
    { id: "blast", label: "Blast Impact Report", category: "Mining Operations", icon: <Bomb className="h-3.5 w-3.5" />, onClick: onOpenBlastReport, domain: "mining" },
    { id: "highwall", label: "Highwall Stability Monitoring", category: "Mining Operations", icon: <ShieldAlert className="h-3.5 w-3.5" />, onClick: onOpenHighwall, domain: "mining" },
    { id: "4d-mon", label: "4D Displacement Monitoring", category: "Mining Operations", icon: <History className="h-3.5 w-3.5" />, onClick: onOpenMonitoring, domain: "mining" },
    { id: "machine-ctrl", label: "Machine Control Compiler", category: "Mining Operations", icon: <Cpu className="h-3.5 w-3.5" />, onClick: onOpenMachineControl, domain: "mining" },
    { id: "ml-class", label: "ML Classification", category: "Mining Operations", icon: <Brain className="h-3.5 w-3.5" />, onClick: onOpenMl, domain: "mining" },
    { id: "setout", label: "Setting Out & Markout", category: "Mining Operations", icon: <Crosshair className="h-3.5 w-3.5" />, onClick: onOpenSetout, domain: "mining" },
    { id: "mine-grid", label: "Mine Grid Transform", category: "Mining Operations", icon: <Grid3x3 className="h-3.5 w-3.5" />, onClick: onOpenMineGrid, domain: "mining" },
    { id: "tunnel", label: "Tunnel Profile Analyzer", category: "Mining Operations", icon: <SquareDashed className="h-3.5 w-3.5" />, onClick: onOpenTunnelProfile, domain: "mining" },
    { id: "safety-rpt", label: "Safety Inspection Report", category: "Mining Operations", icon: <ShieldAlert className="h-3.5 w-3.5" />, onClick: onOpenSafetyReport, domain: "mining" },
    { id: "density-gates", label: "Density Gates Filtering", category: "Mining Operations", icon: <Activity className="h-3.5 w-3.5" />, onClick: onOpenDensityGates, domain: "mining" },
    // GIS & Geodesy
    { id: "map-layout", label: "Generate Alignment Map Sheet", category: "GIS & Geodesy", icon: <MapIcon className="h-3.5 w-3.5" />, onClick: onOpenMapLayout },
    { id: "orthomosaic", label: "Load Orthomosaic (GeoTIFF)", category: "GIS & Geodesy", icon: <Satellite className="h-3.5 w-3.5" />, onClick: onPickOrthomosaic },
    { id: "shapefile", label: "Shapefile Import", category: "GIS & Geodesy", icon: <FileSearch className="h-3.5 w-3.5" />, onClick: onOpenShapefile },
    { id: "idw", label: "IDW Surface Interpolation", category: "GIS & Geodesy", icon: <TrendingUp className="h-3.5 w-3.5" />, onClick: onOpenIdw },
    { id: "topology", label: "Topology Validator", category: "GIS & Geodesy", icon: <ShieldCheck className="h-3.5 w-3.5" />, onClick: onOpenTopology },
    { id: "cross-section", label: "Cross-Section Profiler", category: "GIS & Geodesy", icon: <Ruler className="h-3.5 w-3.5" />, onClick: onOpenCrossSection },
    { id: "slice-3d", label: "3D Slice Editor", category: "GIS & Geodesy", icon: <Scissors className="h-3.5 w-3.5" />, onClick: onOpenSliceEditor },
    { id: "deliverable", label: "Deliverable Package", category: "GIS & Geodesy", icon: <Package className="h-3.5 w-3.5" />, onClick: onOpenDeliverable },
    // Automation & System
    { id: "pipeline-ed", label: "Processing Pipeline Editor", category: "Automation & System", icon: <GitBranch className="h-3.5 w-3.5" />, onClick: onOpenPipeline },
    { id: "ntrip", label: "NTRIP Differential Client", category: "Automation & System", icon: <Radio className="h-3.5 w-3.5" />, onClick: onOpenNtrip },
    { id: "rover-stream", label: "RTK Rover Stream", category: "Automation & System", icon: <Satellite className="h-3.5 w-3.5" />, onClick: onOpenRoverStream },
    { id: "license", label: "License Manager", category: "Automation & System", icon: <Key className="h-3.5 w-3.5" />, onClick: onOpenLicense },
    { id: "benchmark", label: "Hardware Benchmark", category: "Automation & System", icon: <Gauge className="h-3.5 w-3.5" />, onClick: onOpenBenchmark },
    { id: "telemetry", label: "Diagnostics & Telemetry", category: "Automation & System", icon: <Activity className="h-3.5 w-3.5" />, onClick: onOpenTelemetry },
    { id: "marketplace", label: "Plugin Marketplace", category: "Automation & System", icon: <Package className="h-3.5 w-3.5" />, onClick: onOpenMarketplace },
    { id: "update", label: "Check for Updates", category: "Automation & System", icon: <RefreshCw className="h-3.5 w-3.5" />, onClick: onOpenUpdate },
    { id: "account", label: "Surveyor Profile", category: "Automation & System", icon: <User className="h-3.5 w-3.5" />, onClick: onOpenAccount },
  ], [
    onOpenProject, onOpenTriage, onOpenPipelineInspection, onOpenObnSurvey, onOpenCube, onOpenCubeDisambig,
    onOpenS44, onOpenS57, onOpenSvp, onOpenVesselConfig, onOpenSss, onOpenMbesSurvey, onOpenQcDashboard,
    onOpenBackscatter, onOpenTidalDatum, onOpenTidalSpline, onOpenTideGauge, onOpenDredgeAudit, onOpenOdm,
    onOpenCsf, onOpenVolumeCalc, onOpenEomAuditor, onOpenStockpileAudit, onOpenStockpileChange, onOpenBlastReport,
    onOpenHighwall, onOpenMonitoring, onOpenMachineControl, onOpenMl, onOpenSetout, onOpenMineGrid,
    onOpenTunnelProfile, onOpenSafetyReport, onOpenDensityGates, onOpenMapLayout, onPickOrthomosaic,
    onOpenShapefile, onOpenIdw, onOpenTopology, onOpenCrossSection, onOpenSliceEditor, onOpenDeliverable,
    onOpenPipeline, onOpenNtrip, onOpenRoverStream, onOpenLicense, onOpenBenchmark, onOpenTelemetry,
    onOpenMarketplace, onOpenUpdate, onOpenAccount,
  ]);

  const filteredTools = useMemo(() => {
    if (!filterQuery.trim()) return null;
    const q = filterQuery.toLowerCase();
    return allTools.filter((t) => t.label.toLowerCase().includes(q) || t.category.toLowerCase().includes(q));
  }, [allTools, filterQuery]);

  return (
    <aside
      className={`sidebar-transition flex h-full flex-col bg-[#111827] border-r border-[#263345] text-slate-200 select-none ${
        railMode ? "sidebar-rail w-14" : "w-[270px]"
      }`}
    >
      <div className="border-b border-[#263345] p-2.5 flex flex-col gap-2">
        <div className="flex items-center gap-2">
          {railMode ? (
            <button
              title="New Survey Project"
              aria-label="New Survey Project"
              onClick={onOpenProject}
              className="flex h-8 w-8 items-center justify-center rounded-md bg-[#F97316] text-[#0B0F17] hover:bg-[#FB923C] transition-all shadow-sm flex-shrink-0"
            >
              <Plus className="h-4 w-4 stroke-[2.5]" />
            </button>
          ) : (
            <button
              onClick={onOpenProject}
              className="flex flex-1 items-center justify-center gap-2 rounded-md bg-[var(--color-accent)] py-2 px-3 text-xs font-semibold text-[var(--color-base)] shadow-sm transition-colors hover:bg-[var(--color-accent-light)]"
            >
              <Plus className="h-3.5 w-3.5 stroke-[2.5]" />
              New Survey Project
            </button>
          )}
          {onDismiss && (
            <button
              onClick={onDismiss}
              title="Close sidebar"
              aria-label="Close sidebar"
              className="rounded p-1 text-slate-400 hover:bg-[#1F2937] hover:text-white transition-colors"
            >
              <X className="h-4 w-4" />
            </button>
          )}
        </div>

        {/* Quick Tool Search Filter */}
        {!railMode && (
          <div className="relative">
            <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-slate-500 pointer-events-none" />
            <input
              type="text"
              value={filterQuery}
              onChange={(e) => setFilterQuery(e.target.value)}
              placeholder="Filter survey tools..."
              className="w-full rounded-md border border-[#263345] bg-[#0B0F17]/90 pl-8 pr-7 py-1 text-[11px] text-slate-200 placeholder-slate-500 focus:border-[#F97316]/70 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-[var(--color-accent-light)] transition-colors"
            />
            {filterQuery && (
              <button
                onClick={() => setFilterQuery("")}
                className="absolute right-2 top-1/2 -translate-y-1/2 text-slate-500 hover:text-slate-300 text-xs px-1"
                aria-label="Clear filter"
              >
                ✕
              </button>
            )}
          </div>
        )}
      </div>

      <div className="flex-1 overflow-y-auto p-2">
        {filteredTools ? (
          <div className="space-y-1">
            <div className="px-2 py-1 text-[10px] font-semibold uppercase tracking-wider text-slate-400">
              Matches ({filteredTools.length})
            </div>
            {filteredTools.length === 0 ? (
              <div className="px-3 py-4 text-center text-xs text-slate-500">
                No tools match &ldquo;{filterQuery}&rdquo;
              </div>
            ) : (
              filteredTools.map((tool) => (
                <SidebarItem
                  key={tool.id}
                  icon={tool.icon}
                  label={tool.label}
                  badge={tool.badge}
                  onClick={() => {
                    tool.onClick();
                    setFilterQuery("");
                  }}
                />
              ))
            )}
          </div>
        ) : (
          <>
            {/* ── Project ── */}
            <SidebarSection title="Project" icon={<Folder className="h-3 w-3" />}>
              <SidebarItem icon={<FileBox className="h-3 w-3" />} label="Project Manager" onClick={onOpenProject} />
              <SidebarItem icon={<FolderOpen className="h-3 w-3" />} label="Mission Triage" onClick={onOpenTriage} />
            </SidebarSection>

            {/* ── Marine & Subsea ── */}
            {domain !== "mining" && (
              <SidebarSection title="Marine & Subsea" icon={<Waves className="h-3 w-3 text-teal-400" />}>
                <SidebarItem icon={<GitBranch className="h-3 w-3" />} label="Pipeline Inspection (EIVA / DXF)" badge="DNV" onClick={onOpenPipelineInspection} />
                <SidebarItem icon={<Radio className="h-3 w-3" />} label="OBN Seismic Survey (4DNav)" badge="P1/11" onClick={onOpenObnSurvey} />
                <SidebarItem icon={<Waves className="h-3 w-3" />} label="CUBE Surface" onClick={onOpenCube} />
                <SidebarItem icon={<Layers3 className="h-3 w-3" />} label="CUBE Disambiguation" onClick={onOpenCubeDisambig} />
                <SidebarItem icon={<Shield className="h-3 w-3" />} label="S-44 Compliance" onClick={onOpenS44} />
                <SidebarItem icon={<Anchor className="h-3 w-3" />} label="S-57 Export" onClick={onOpenS57} />
                <SidebarItem icon={<Waves className="h-3 w-3" />} label="SVP Sound Velocity" onClick={onOpenSvp} />
                <SidebarItem icon={<Anchor className="h-3 w-3" />} label="Vessel Configuration" onClick={onOpenVesselConfig} />
                <SidebarItem icon={<Waves className="h-3 w-3" />} label="Side-Scan Sonar (SSS)" onClick={onOpenSss} />
                <SidebarItem icon={<FileSearch className="h-3 w-3" />} label="MBES Survey Reader (.all)" onClick={onOpenMbesSurvey} />
                <SidebarItem icon={<Activity className="h-3 w-3" />} label="Real-Time QC Dashboard" onClick={onOpenQcDashboard} />
                <SidebarItem icon={<Grid3x3 className="h-3 w-3" />} label="Backscatter Mosaic" onClick={onOpenBackscatter} />
                <SidebarItem icon={<Waves className="h-3 w-3" />} label="Tidal Datum Converter" onClick={onOpenTidalDatum} />
                <SidebarItem icon={<Waves className="h-3 w-3" />} label="Tidal Spline Corrector" onClick={onOpenTidalSpline} />
                <SidebarItem icon={<Waves className="h-3 w-3" />} label="Tide Gauge (NOAA / TCP)" onClick={onOpenTideGauge} />
                <SidebarItem icon={<Waves className="h-3 w-3" />} label="Dredge Progress Audit" onClick={onOpenDredgeAudit} />
              </SidebarSection>
            )}

            {/* ── Mining Operations ── */}
            {domain !== "marine" && (
              <SidebarSection title="Mining Operations" icon={<Activity className="h-3 w-3 text-amber-400" />}>
                <SidebarItem icon={<Terminal className="h-3 w-3" />} label="ODM Photogrammetry" onClick={onOpenOdm} />
                <SidebarItem icon={<Layers3 className="h-3 w-3" />} label="Classify Ground (CSF)" onClick={onOpenCsf} />
                <SidebarItem icon={<Calculator className="h-3 w-3" />} label="Volume Calculator" onClick={onOpenVolumeCalc} />
                <SidebarItem icon={<ShieldCheck className="h-3 w-3" />} label="EOM Volumetric Auditor" badge="Audit" onClick={onOpenEomAuditor} />
                <SidebarItem icon={<Boxes className="h-3 w-3" />} label="Stockpile Audit" onClick={onOpenStockpileAudit} />
                <SidebarItem icon={<History className="h-3 w-3" />} label="Stockpile Change Detection" onClick={onOpenStockpileChange} />
                <SidebarItem icon={<Bomb className="h-3 w-3" />} label="Blast Impact Report" onClick={onOpenBlastReport} />
                <SidebarItem icon={<ShieldAlert className="h-3 w-3" />} label="Highwall Monitoring" onClick={onOpenHighwall} />
                <SidebarItem icon={<History className="h-3 w-3" />} label="4D Displacement Tracking" onClick={onOpenMonitoring} />
                <SidebarItem icon={<Cpu className="h-3 w-3" />} label="Machine Control Compiler" onClick={onOpenMachineControl} />
                <SidebarItem icon={<Brain className="h-3 w-3" />} label="ML Point Classification" onClick={onOpenMl} />
                <SidebarItem icon={<Crosshair className="h-3 w-3" />} label="Setting Out & Markout" onClick={onOpenSetout} />
                <SidebarItem icon={<Grid3x3 className="h-3 w-3" />} label="Mine Grid Transform" onClick={onOpenMineGrid} />
                <SidebarItem icon={<SquareDashed className="h-3 w-3" />} label="Tunnel Profile Analyzer" onClick={onOpenTunnelProfile} />
                <SidebarItem icon={<ShieldAlert className="h-3 w-3" />} label="Safety Inspection Report" onClick={onOpenSafetyReport} />
                <SidebarItem icon={<Activity className="h-3 w-3" />} label="Density Gates Filtering" onClick={onOpenDensityGates} />
              </SidebarSection>
            )}

            {/* ── GIS & Geodesy ── */}
            <SidebarSection title="GIS & Geodesy" icon={<Grid3x3 className="h-3 w-3" />} defaultOpen={false}>
              <SidebarItem icon={<MapIcon className="h-3 w-3" />} label="Generate Alignment Map Sheet" onClick={onOpenMapLayout} />
              <SidebarItem icon={<Satellite className="h-3 w-3" />} label="Load Orthomosaic (GeoTIFF)" onClick={onPickOrthomosaic} />
              <SidebarItem icon={<FileSearch className="h-3 w-3" />} label="Shapefile Import" onClick={onOpenShapefile} />
              <SidebarItem icon={<TrendingUp className="h-3 w-3" />} label="IDW Surface Interpolation" onClick={onOpenIdw} />
              <SidebarItem icon={<ShieldCheck className="h-3 w-3" />} label="Topology Validator" onClick={onOpenTopology} />
              <SidebarItem icon={<Ruler className="h-3 w-3" />} label="Cross-Section Profiler" onClick={onOpenCrossSection} />
              <SidebarItem icon={<Scissors className="h-3 w-3" />} label="3D Slice Editor" onClick={onOpenSliceEditor} />
              <SidebarItem icon={<Package className="h-3 w-3" />} label="Client Deliverable Package" onClick={onOpenDeliverable} />
            </SidebarSection>

            {/* ── Automation & System ── */}
            <SidebarSection title="Automation & System" icon={<Shield className="h-3 w-3" />} defaultOpen={false}>
              <SidebarItem icon={<GitBranch className="h-3 w-3" />} label="Pipeline Editor" onClick={onOpenPipeline} />
              <SidebarItem icon={<Radio className="h-3 w-3" />} label="NTRIP Differential Client" onClick={onOpenNtrip} />
              <SidebarItem icon={<Satellite className="h-3 w-3" />} label="RTK Rover Stream" onClick={onOpenRoverStream} />
              <SidebarItem icon={<Key className="h-3 w-3" />} label="License Manager" onClick={onOpenLicense} />
              <SidebarItem icon={<Gauge className="h-3 w-3" />} label="Hardware Benchmark" onClick={onOpenBenchmark} />
              <SidebarItem icon={<Activity className="h-3 w-3" />} label="Diagnostics & Telemetry" onClick={onOpenTelemetry} />
              <SidebarItem icon={<Package className="h-3 w-3" />} label="Plugin Marketplace" onClick={onOpenMarketplace} />
              <SidebarItem icon={<RefreshCw className="h-3 w-3" />} label="Check for Updates" onClick={onOpenUpdate} />
              <SidebarItem icon={<User className="h-3 w-3" />} label="Surveyor Profile" onClick={onOpenAccount} />
            </SidebarSection>
          </>
        )}
      </div>

      <div className="border-t border-[#263345] p-2 bg-[#0B0F17]/60 space-y-0.5">
        <SidebarItem icon={<Settings className="h-3.5 w-3.5" />} label="Preferences & Settings" onClick={onOpenSettings} />
        <SidebarItem icon={<Keyboard className="h-3.5 w-3.5" />} label="Keyboard Shortcuts (?)" onClick={onOpenShortcuts} />
        <SidebarItem icon={<Bookmark className="h-3.5 w-3.5" />} label="Saved Views" onClick={onOpenSavedViews} />
        <SidebarItem icon={<SunMoon className="h-3.5 w-3.5" />} label="Toggle Daylight Theme" onClick={onToggleTheme} />
        <SidebarItem icon={<Eye className="h-3.5 w-3.5" />} label="Colorblind Accessibility" onClick={onToggleColorblind} />
      </div>
    </aside>
  );
}

function SidebarSection({
  title,
  icon,
  badge,
  defaultOpen = true,
  children,
}: {
  title: string;
  icon: React.ReactNode;
  badge?: string | number;
  defaultOpen?: boolean;
  children: React.ReactNode;
}) {
  const storageKey = "metardu.sidebar.sections";
  const [open, setOpen] = useState(() => {
    if (typeof window === "undefined") return defaultOpen;
    try {
      const stored = JSON.parse(window.localStorage.getItem(storageKey) ?? "{}") as Record<string, boolean>;
      return stored[title] ?? defaultOpen;
    } catch {
      return defaultOpen;
    }
  });

  useEffect(() => {
    try {
      const stored = JSON.parse(window.localStorage.getItem(storageKey) ?? "{}") as Record<string, boolean>;
      window.localStorage.setItem(storageKey, JSON.stringify({ ...stored, [title]: open }));
    } catch {
      // Sidebar grouping is an enhancement; a blocked storage backend should not affect navigation.
    }
  }, [open, title]);

  return (
    <div className="mb-2">
      <button
        onClick={() => setOpen((v) => !v)}
        className="flex w-full items-center justify-between px-2 py-1 text-[10px] font-semibold uppercase tracking-wider text-slate-400 hover:text-slate-200 transition-colors"
      >
        <div className="flex items-center gap-1.5">
          <span className="text-slate-400">{icon}</span>
          <span>{title}</span>
          {badge !== undefined && (
            <span className="rounded-full bg-[#1F2937] px-1.5 py-0.2 text-[9px] font-mono text-slate-400">
              {badge}
            </span>
          )}
        </div>
        <ChevronRight
          className={`h-3 w-3 text-slate-500 transition-transform duration-150 ${
            open ? "rotate-90" : ""
          }`}
        />
      </button>
      {open && <div className="mt-0.5 space-y-0.5 pl-1">{children}</div>}
    </div>
  );
}

function SidebarItem({
  icon,
  label,
  active = false,
  indent = false,
  badge,
  onClick,
}: {
  icon?: React.ReactNode;
  label: string;
  active?: boolean;
  indent?: boolean;
  badge?: string;
  onClick?: () => void;
}) {
  return (
    <button
      onClick={onClick}
      title={label}
      className={`group flex w-full items-center gap-2.5 rounded-md px-2 py-1.5 text-[12px] font-medium transition-all ${
        active
          ? "bg-[#1F2937] text-white font-semibold shadow-sm"
          : "text-slate-300 hover:bg-[#1F2937]/70 hover:text-white"
      } ${indent ? "pl-5" : ""}`}
    >
      {icon && (
        <span
          className={`flex-shrink-0 transition-colors ${
            active ? "text-[#F97316]" : "text-slate-400 group-hover:text-slate-200"
          }`}
        >
          {icon}
        </span>
      )}
      <span className="flex-1 text-left truncate">{label}</span>
      {badge && (
        <span className="rounded bg-[#1F2937] px-1.5 py-0.2 text-[9px] font-mono text-slate-400 border border-[#263345]">
          {badge}
        </span>
      )}
      {active && (
        <span className="h-1.5 w-1.5 rounded-full bg-[#F97316] flex-shrink-0" />
      )}
    </button>
  );
}

/* ──────────────────────────────────────────────────────────── */
