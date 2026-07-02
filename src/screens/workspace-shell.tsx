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

import { useCallback, useState } from "react";
import type Map from "ol/Map";
import {
  Folder,
  FileBox,
  Layers,
  Database,
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
} from "lucide-react";
import { MapCanvas } from "@/components/map-canvas";
import { FileDropOverlay } from "@/components/file-drop-overlay";
import { CrsSwitchBanner } from "@/components/crs-switch-banner";
import { SettingsDialog } from "@/components/settings-dialog";
import { ProfilePanel } from "@/components/profile-panel";
import { VolumeCalcDialog } from "@/components/volume-calc-dialog";
import { useProfileTool, type ProfileLine } from "@/lib/use-profile-tool";
import {
  colors,
  domainAccent,
  APP_NAME,
  APP_VERSION,
  type DomainMode,
} from "@/lib/tokens";
import { useAppStore } from "@/stores/app-store";
import { useSurveyStore } from "@/stores/survey-store";

export function WorkspaceShell() {
  const { activeDomain, settings } = useAppStore();
  const [sidebarOpen, setSidebarOpen] = useState(true);
  const [rightPanelOpen, setRightPanelOpen] = useState(true);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [volumeCalcOpen, setVolumeCalcOpen] = useState(false);
  const [mapInstance, setMapInstance] = useState<Map | null>(null);
  const [profileActive, setProfileActive] = useState(false);

  const handleMapReady = useCallback((map: Map) => {
    setMapInstance(map);
  }, []);

  const { line: profileLine, clear: clearProfile } = useProfileTool({
    map: mapInstance,
    active: profileActive,
    domain: activeDomain,
  });

  return (
    <div className="flex h-full w-full flex-col bg-navy-base">
      <TitleBar domain={activeDomain} />
      <div className="flex flex-1 overflow-hidden">
        {sidebarOpen && (
          <LeftSidebar
            domain={activeDomain}
            onOpenSettings={() => setSettingsOpen(true)}
            onOpenVolumeCalc={() => setVolumeCalcOpen(true)}
          />
        )}
        <main className="relative flex-1 overflow-hidden">
          <MapCanvas
            domain={activeDomain}
            epsg={settings.defaultEpsg}
            onMapReady={handleMapReady}
          />
          <FileDropOverlay domain={activeDomain} />
          <CrsSwitchBanner />
          <FloatingActions
            onToggleSidebar={() => setSidebarOpen((v) => !v)}
            onToggleRight={() => setRightPanelOpen((v) => !v)}
            onOpenSettings={() => setSettingsOpen(true)}
            onOpenVolumeCalc={() => setVolumeCalcOpen(true)}
            profileActive={profileActive}
            onToggleProfile={() => setProfileActive((v) => !v)}
          />
          {profileActive && (
            <div
              className="pointer-events-none absolute left-1/2 top-12 z-30 -translate-x-1/2 rounded-md border px-3 py-1.5 text-[11px] backdrop-blur"
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
            profileLine={profileLine}
            onClearProfile={clearProfile}
            profileActive={profileActive}
          />
        )}
      </div>
      <StatusBar domain={activeDomain} epsg={settings.defaultEpsg} />
      <SettingsDialog open={settingsOpen} onClose={() => setSettingsOpen(false)} />
      <VolumeCalcDialog open={volumeCalcOpen} onClose={() => setVolumeCalcOpen(false)} />
    </div>
  );
}

/* ──────────────────────────────────────────────────────────── */

function TitleBar({ domain }: { domain: DomainMode }) {
  const accent = domainAccent[domain].primary;
  return (
    <header className="flex h-10 items-center justify-between border-b border-navy-border bg-navy-panel px-3">
      <div className="flex items-center gap-3">
        <div
          className="flex h-6 w-6 items-center justify-center rounded text-xs font-bold"
          style={{ background: colors.industrialOrange, color: colors.navyBase }}
        >
          M
        </div>
        <span className="text-xs font-semibold tracking-wide text-white">
          {APP_NAME}
        </span>
        <span className="text-steel-gray">/</span>
        <span className="text-xs text-steel-light">Untitled Project</span>
        <span
          className="rounded-sm px-1.5 py-0.5 text-[9px] font-semibold uppercase tracking-wider"
          style={{
            background: `${accent}20`,
            color: accent,
            border: `1px solid ${accent}40`,
          }}
        >
          {domainAccent[domain].label}
        </span>
      </div>

      <div className="flex items-center gap-1">
        <button className="rounded p-1 text-steel-gray hover:bg-navy-elevated hover:text-white">
          <Minus className="h-3.5 w-3.5" />
        </button>
        <button className="rounded p-1 text-steel-gray hover:bg-navy-elevated hover:text-white">
          <Square className="h-3 w-3" />
        </button>
        <button className="rounded p-1 text-steel-gray hover:bg-fail/20 hover:text-fail">
          <X className="h-3.5 w-3.5" />
        </button>
      </div>
    </header>
  );
}

/* ──────────────────────────────────────────────────────────── */

function LeftSidebar({
  domain,
  onOpenSettings,
  onOpenVolumeCalc,
}: {
  domain: DomainMode;
  onOpenSettings: () => void;
  onOpenVolumeCalc: () => void;
}) {
  const accent = domainAccent[domain].primary;

  return (
    <aside className="flex w-60 flex-col border-r border-navy-border bg-navy-panel">
      <div className="border-b border-navy-border p-3">
        <button
          className="flex w-full items-center justify-center gap-2 rounded-md py-2 text-xs font-medium transition-colors"
          style={{ background: accent, color: colors.navyBase }}
        >
          <Plus className="h-3.5 w-3.5" />
          New Survey
        </button>
      </div>

      <div className="flex-1 overflow-y-auto p-2">
        <SidebarSection title="Project" icon={<Folder className="h-3 w-3" />}>
          <SidebarItem icon={<FileBox className="h-3 w-3" />} label="Untitled Project" active />
          <SidebarItem icon={<Layers className="h-3 w-3" />} label="Layers" indent />
          <SidebarItem icon={<Database className="h-3 w-3" />} label="Data sources" indent />
        </SidebarSection>

        {domain !== "marine" && (
          <SidebarSection title="Mining" icon={<Activity className="h-3 w-3" />}>
            <SidebarItem label="UAV Surveys" indent />
            <SidebarItem label="TLS Stations" indent />
            <SidebarItem label="Stockpiles" indent />
            <SidebarItem label="Blast Designs" indent />
            <SidebarItem label="4D Monitoring" indent />
            <div className="my-1.5 border-t border-navy-border" />
            <SidebarItem
              icon={<Calculator className="h-3 w-3" />}
              label="Volume Calculator"
              onClick={onOpenVolumeCalc}
            />
          </SidebarSection>
        )}

        {domain !== "mining" && (
          <SidebarSection title="Marine" icon={<Activity className="h-3 w-3" />}>
            <SidebarItem label="Survey Lines" indent />
            <SidebarItem label="SVP Casts" indent />
            <SidebarItem label="Tide Gauges" indent />
            <SidebarItem label="CUBE Surfaces" indent />
            <SidebarItem label="S-44 Reports" indent />
          </SidebarSection>
        )}
      </div>

      <div className="border-t border-navy-border p-2">
        <SidebarItem
          icon={<Settings className="h-3 w-3" />}
          label="Settings"
          onClick={onOpenSettings}
        />
        <SidebarItem icon={<HelpCircle className="h-3 w-3" />} label="Help & Docs" />
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
      <div className="flex items-center gap-1.5 px-2 py-1.5 text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
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
      className={`flex w-full items-center gap-2 rounded-md py-1.5 text-xs transition-colors ${
        active
          ? "bg-navy-elevated text-white"
          : "text-steel-light hover:bg-navy-elevated/50 hover:text-white"
      } ${indent ? "pl-6" : "pl-2"} pr-2`}
    >
      {icon && <span className="text-steel-gray">{icon}</span>}
      <span className="flex-1 text-left">{label}</span>
      {active && (
        <ChevronRight className="h-3 w-3" style={{ color: colors.industrialOrange }} />
      )}
    </button>
  );
}

/* ──────────────────────────────────────────────────────────── */

function RightPanel({
  domain,
  profileLine,
  onClearProfile,
  profileActive,
}: {
  domain: DomainMode;
  profileLine: ProfileLine | null;
  onClearProfile: () => void;
  profileActive: boolean;
}) {
  const accent = domainAccent[domain].primary;
  const files = useSurveyStore((s) => s.files);
  const activeFileId = useSurveyStore((s) => s.activeFileId);
  const setActiveFile = useSurveyStore((s) => s.setActiveFile);
  const removeFile = useSurveyStore((s) => s.removeFile);

  const fileCount = files.length;
  const loadedCount = files.filter((f) => f.status === "loaded").length;

  return (
    <aside className="flex w-80 flex-col border-l border-navy-border bg-navy-panel">
      <div className="border-b border-navy-border px-4 py-3">
        <h3 className="text-xs font-semibold uppercase tracking-wider text-steel-light">
          {domain === "marine" ? "S-44 Status" : "Survey Status"}
        </h3>
      </div>

      <div className="flex-1 overflow-y-auto p-4">
        {/* Status block */}
        <div className="rounded-lg border border-navy-border bg-navy-base p-4">
          <div className="flex items-center justify-between">
            <span className="text-xs text-steel-gray">Survey ready</span>
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
              {fileCount > 0 ? `${loadedCount}/${fileCount} loaded` : "No Data"}
            </span>
          </div>
          <div className="mt-3 text-2xl font-bold text-white">
            {fileCount > 0 ? fileCount : "—"}
          </div>
          <div className="text-xs text-steel-gray">
            {domain === "marine" ? "Files staged" : "Files staged"}
          </div>
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
          <StatTile label="CRS" value="EPSG:4326" mono />
          <StatTile label="Domain" value={domainAccent[domain].label} />
          <StatTile
            label={domain === "marine" ? "S-44 Order" : "Bench Level"}
            value="—"
          />
          <StatTile label="Last sync" value="—" />
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
              ? domain === "marine"
                ? "Drop a Kongsberg .all or Reson .s7k file anywhere to start the ingest pipeline."
                : "Drop a LAS/LAZ point cloud or drone photogrammetry export to begin."
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
}: {
  onToggleSidebar: () => void;
  onToggleRight: () => void;
  onOpenSettings: () => void;
  onOpenVolumeCalc: () => void;
  profileActive: boolean;
  onToggleProfile: () => void;
}) {
  return (
    <div className="absolute right-3 top-3 flex flex-col gap-1">
      <button
        onClick={onToggleRight}
        title="Toggle right panel"
        className="rounded border border-navy-border bg-navy-base/85 p-1.5 text-steel-light backdrop-blur hover:bg-navy-elevated hover:text-white"
      >
        <Square className="h-3 w-3" />
      </button>
      <button
        onClick={onToggleSidebar}
        title="Toggle sidebar"
        className="rounded border border-navy-border bg-navy-base/85 p-1.5 text-steel-light backdrop-blur hover:bg-navy-elevated hover:text-white"
      >
        <Layers className="h-3 w-3" />
      </button>
      <button
        onClick={onToggleProfile}
        title="Profile tool"
        className="rounded border p-1.5 backdrop-blur transition-colors"
        style={{
          background: profileActive ? colors.industrialOrange : "rgba(10, 25, 47, 0.85)",
          borderColor: profileActive ? colors.industrialOrange : colors.navyBorder,
          color: profileActive ? colors.navyBase : colors.steelLight,
        }}
      >
        <TrendingUp className="h-3 w-3" />
      </button>
      <button
        onClick={onOpenVolumeCalc}
        title="Volume calculator"
        className="rounded border border-navy-border bg-navy-base/85 p-1.5 text-steel-light backdrop-blur hover:bg-navy-elevated hover:text-white"
      >
        <Calculator className="h-3 w-3" />
      </button>
      <button
        onClick={onOpenSettings}
        title="Settings"
        className="rounded border border-navy-border bg-navy-base/85 p-1.5 text-steel-light backdrop-blur hover:bg-navy-elevated hover:text-white"
      >
        <Settings className="h-3 w-3" />
      </button>
    </div>
  );
}

/* ──────────────────────────────────────────────────────────── */

function StatusBar({ domain, epsg }: { domain: DomainMode; epsg: string }) {
  const accent = domainAccent[domain].primary;
  return (
    <footer className="flex h-6 items-center justify-between border-t border-navy-border bg-navy-panel px-3 text-[10px]">
      <div className="flex items-center gap-4">
        <span className="flex items-center gap-1.5 text-steel-light">
          <span
            className="h-1.5 w-1.5 rounded-full"
            style={{ background: colors.pass }}
          />
          Ready
        </span>
        <span className="flex items-center gap-1 text-steel-gray">
          <MapPin className="h-3 w-3" style={{ color: accent }} />
          <span className="font-mono text-steel-light">{epsg}</span>
        </span>
        <span className="flex items-center gap-1 text-steel-gray">
          <Crosshair className="h-3 w-3" />
          <span style={{ color: accent }}>{domainAccent[domain].label}</span>
        </span>
      </div>
      <div className="flex items-center gap-4 text-steel-gray">
        <span className="flex items-center gap-1">
          <Clock className="h-3 w-3" />
          <span className="font-mono">{new Date().toISOString().slice(11, 19)}Z</span>
        </span>
        <span className="font-mono">v{APP_VERSION}</span>
      </div>
    </footer>
  );
}
