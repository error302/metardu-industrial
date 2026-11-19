import { ChevronRight, X } from "lucide-react";
import { ProfilePanel } from "@/components/profile-panel";
import { domainAccent, type DomainMode } from "@/lib/tokens";
import type { ProfileLine } from "@/lib/use-profile-tool";
import { useSurveyStore } from "@/stores/survey-store";

interface SurveyBriefActions {
  classify: () => void;
  volume: () => void;
  audit: () => void;
  cube: () => void;
  s44: () => void;
  deliver: () => void;
}

interface SurveyBriefPanelProps {
  domain: DomainMode;
  epsg: string;
  profileLine: ProfileLine | null;
  profileActive: boolean;
  onClearProfile: () => void;
  actions: SurveyBriefActions;
}

export function SurveyBriefPanel({
  domain,
  epsg,
  profileLine,
  profileActive,
  onClearProfile,
  actions,
}: SurveyBriefPanelProps) {
  const accent = domainAccent[domain].primary;
  const files = useSurveyStore((state) => state.files);
  const activeFileId = useSurveyStore((state) => state.activeFileId);
  const setActiveFile = useSurveyStore((state) => state.setActiveFile);
  const removeFile = useSurveyStore((state) => state.removeFile);

  const fileCount = files.length;
  const loadedCount = files.filter((file) => file.status === "loaded").length;
  const errorCount = files.filter((file) => file.status === "error").length;
  const probingCount = files.filter((file) => file.status === "probing" || file.status === "pending").length;
  const totalSize = files.reduce((sum, file) => sum + file.size, 0);
  const totalPoints = files.filter((file) => file.kind === "las").reduce((sum, file) => sum + (file.pointCount ?? 0), 0);
  const lastAddedAt = files.reduce((latest, file) => Math.max(latest, file.addedAt), 0);
  const hasBounds = files.some((file) => file.bounds);
  const statusLabel =
    fileCount === 0
      ? "Ready for ingest"
      : errorCount > 0
        ? "Needs review"
        : probingCount > 0
          ? "Probing headers"
          : "Headers inspected";
  const nextActions = getNextActions(domain, actions);
  const emptyTip =
    domain === "marine"
      ? "Drop a Kongsberg .all or Reson .s7k file anywhere to start the ingest pipeline."
      : domain === "mining"
        ? "Drop a LAS/LAZ point cloud or drone photogrammetry export to begin."
        : "Drop LAS/LAZ, GeoTIFF, MBES, side-scan, drone manifest, or control files to begin.";

  return (
    <aside
      aria-label="Survey brief"
      className="sidebar-transition flex w-72 flex-col border-l border-[var(--color-border)] bg-[var(--color-panel)] text-[var(--color-white)] xl:w-80"
    >
      <div className="flex items-center justify-between gap-2 border-b border-[var(--color-border)] px-4 py-2.5">
        <h2 className="text-xs font-semibold uppercase tracking-wider text-[var(--color-text-secondary)]">Survey brief</h2>
        <span
          className="truncate rounded-sm border border-[var(--color-border)] bg-[var(--color-elevated)] px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-wider"
          style={{ color: accent }}
        >
          {domainAccent[domain].label}
        </span>
      </div>

      <div className="flex-1 space-y-4 overflow-y-auto p-4">
        <section className="rounded-md border border-[var(--color-border)] bg-[var(--color-base)] p-3.5">
          <div className="flex items-center justify-between gap-2">
            <span className="text-xs font-medium text-[var(--color-text-secondary)]">{statusLabel}</span>
            <span
              className="rounded-sm px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wider"
              style={
                errorCount > 0
                  ? { background: "color-mix(in srgb, var(--color-fail) 15%, transparent)", color: "var(--color-fail)" }
                  : { background: "color-mix(in srgb, var(--color-warn) 15%, transparent)", color: "var(--color-warn)" }
              }
            >
              {fileCount > 0 ? `${loadedCount}/${fileCount} loaded` : "No data"}
            </span>
          </div>
          <div className="mt-2 font-mono text-2xl font-bold tabular-nums">{fileCount || "—"}</div>
          <p className="text-[11px] text-[var(--color-text-secondary)]">staged files</p>
        </section>

        <section className="grid grid-cols-3 gap-2" aria-label="Survey metrics">
          <MetricTile label="Points" value={totalPoints ? compactNumber(totalPoints) : "—"} />
          <MetricTile label="Data" value={totalSize ? formatBytes(totalSize) : "—"} />
          <MetricTile label="Errors" value={String(errorCount)} tone={errorCount > 0 ? "fail" : "pass"} />
        </section>

        {fileCount > 0 && (
          <section>
            <h3 className="mb-2 text-[10px] font-semibold uppercase tracking-wider text-[var(--color-text-secondary)]">Staged files</h3>
            <div className="space-y-1.5">
              {files.map((file) => (
                <div
                  key={file.id}
                  className={`block w-full rounded-md border px-2.5 py-2 text-left transition-colors ${
                    activeFileId === file.id
                      ? "border-amber-500/60 bg-[var(--color-elevated)]"
                      : "border-[var(--color-border)] bg-[var(--color-base)] hover:bg-[var(--color-elevated)]"
                  }`}
                >
                  <span className="flex items-center justify-between gap-2">
                    <button
                      type="button"
                      onClick={() => setActiveFile(file.id)}
                      className="min-w-0 flex-1 truncate text-left text-xs font-medium"
                    >
                      {file.name}
                    </button>
                    <button
                      type="button"
                      aria-label={`Remove ${file.name}`}
                      onClick={() => removeFile(file.id)}
                      className="rounded-sm p-0.5 text-[var(--color-text-secondary)] hover:text-red-400"
                    >
                      <X className="h-3 w-3" />
                    </button>
                  </span>
                  <span className="mt-1 flex items-center justify-between text-[10px] text-[var(--color-text-secondary)]">
                    <span className="rounded-sm border border-[var(--color-border)] bg-[var(--color-panel)] px-1 py-0.5 font-mono uppercase">{file.kind}</span>
                    <span className="font-mono">{formatBytes(file.size)}</span>
                  </span>
                  {file.status === "error" && file.errorMessage && (
                    <span className="mt-1 block text-[10px] text-red-400">{file.errorMessage}</span>
                  )}
                </div>
              ))}
            </div>
          </section>
        )}

        <section className="grid grid-cols-2 gap-2" aria-label="Survey facts">
          <StatTile label="CRS" value={epsg} mono />
          <StatTile label="Domain" value={domainAccent[domain].label} />
          <StatTile label="File bounds" value={hasBounds ? "Available" : "Unavailable"} />
          <StatTile label="Last ingest" value={lastAddedAt ? new Date(lastAddedAt).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }) : "—"} />
        </section>

        <section>
          <h3 className="mb-2 text-[10px] font-semibold uppercase tracking-wider text-[var(--color-text-secondary)]">Recommended next actions</h3>
          <div className="space-y-2">
            {nextActions.map((action) => (
              <button
                key={action.title}
                type="button"
                onClick={action.onClick}
                className="w-full rounded-md border border-[var(--color-border)] bg-[var(--color-base)] p-2.5 text-left transition-colors hover:bg-[var(--color-elevated)]"
              >
                <span className="flex items-center justify-between gap-2">
                  <span className="text-xs font-semibold">{action.title}</span>
                  <ChevronRight className="h-3 w-3 text-[var(--color-text-secondary)]" />
                </span>
                <span className="mt-0.5 block text-[10px] leading-relaxed text-[var(--color-text-secondary)]">{action.body}</span>
              </button>
            ))}
          </div>
        </section>

        <section className="rounded-md border p-3 text-xs" style={{ borderColor: `${accent}40`, background: `${accent}10` }}>
          <h3 className="mb-1 font-semibold" style={{ color: accent }}>Next step</h3>
          <p className="text-[11px] leading-relaxed text-[var(--color-text-secondary)]">
            {fileCount === 0 ? emptyTip : "Select a staged file for inspection. Bounds are shown only when its coordinate system is known."}
          </p>
        </section>
      </div>

      {profileActive && (
        <section className="border-t border-[var(--color-border)]">
          <div className="flex items-center justify-between border-b border-[var(--color-border)] px-3 py-1.5">
            <h3 className="text-[10px] font-semibold uppercase tracking-wider text-[var(--color-text-secondary)]">Profile cross-section</h3>
            {profileLine && <button type="button" onClick={onClearProfile} className="text-[10px] text-[var(--color-text-secondary)] hover:text-[var(--color-white)]">Clear</button>}
          </div>
          <div className="h-44"><ProfilePanel domain={domain} line={profileLine ? [profileLine.start, profileLine.end] : null} /></div>
        </section>
      )}
    </aside>
  );
}

function getNextActions(domain: DomainMode, actions: SurveyBriefActions) {
  if (domain === "mining") {
    return [
      { title: "Classify ground", body: "Run CSF before volume calculations.", onClick: actions.classify },
      { title: "Compute volume", body: "Compare the current survey against a bench or previous survey.", onClick: actions.volume },
      { title: "Audit results", body: "Create a signed volumetric report.", onClick: actions.audit },
    ];
  }
  if (domain === "marine") {
    return [
      { title: "Generate CUBE", body: "Build a defensible bathymetric surface.", onClick: actions.cube },
      { title: "Check S-44", body: "Review compliance and TPU warnings.", onClick: actions.s44 },
      { title: "Package delivery", body: "Prepare outputs and an audit manifest.", onClick: actions.deliver },
    ];
  }
  return [
    { title: "Run mining QC", body: "Classify ground before reporting mine volumes.", onClick: actions.classify },
    { title: "Run marine QC", body: "Build a CUBE surface and check S-44 readiness.", onClick: actions.cube },
    { title: "Package delivery", body: "Prepare reports, S-57 outputs, and audit manifests.", onClick: actions.deliver },
  ];
}

function MetricTile({ label, value, tone = "neutral" }: { label: string; value: string; tone?: "neutral" | "pass" | "fail" }) {
  const color = tone === "pass" ? "var(--color-pass)" : tone === "fail" ? "var(--color-fail)" : "var(--color-white)";
  return <div className="rounded-md border border-[var(--color-border)] bg-[var(--color-base)] p-2"><div className="text-[9px] font-semibold uppercase tracking-wider text-[var(--color-text-secondary)]">{label}</div><div className="mt-0.5 truncate font-mono text-xs font-semibold" style={{ color }}>{value}</div></div>;
}

function StatTile({ label, value, mono = false }: { label: string; value: string; mono?: boolean }) {
  return <div className="rounded-md border border-[var(--color-border)] bg-[var(--color-base)] p-2.5"><div className="text-[9px] font-semibold uppercase tracking-wider text-[var(--color-text-secondary)]">{label}</div><div className={`mt-0.5 truncate text-xs font-medium ${mono ? "font-mono" : ""}`}>{value}</div></div>;
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const index = Math.floor(Math.log(bytes) / Math.log(1024));
  return `${(bytes / Math.pow(1024, index)).toFixed(1)} ${["B", "KB", "MB", "GB"][index]}`;
}

function compactNumber(value: number): string {
  return Intl.NumberFormat(undefined, { notation: "compact", maximumFractionDigits: 1 }).format(value);
}
