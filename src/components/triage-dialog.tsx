/**
 * Mission Data Triage Dialog — field data verification + gap analysis.
 *
 * Surveyor drops a folder of field data (drone images, LAS/LAZ, GNSS logs).
 * The Rust core scans every file in parallel and returns a TriageReport with:
 *   - File health (OK/Warning/Error/Empty)
 *   - Spatial coverage bounds
 *   - CRS mismatches
 *   - Temporal span
 *   - Coverage gaps
 *
 * This prevents the most expensive mistake in field surveying: driving back
 * to a remote site because of a coverage gap discovered days later.
 */

import { useState, useCallback } from "react";
import { useEscapeKey } from "@/lib/use-escape-key";
import {
  X, FolderOpen, Loader2, CheckCircle2, AlertTriangle, AlertCircle,
  FileText, MapPin, Clock, HardDrive, Database, Radio, FileBox,
} from "lucide-react";
import { colors } from "@/lib/tokens";
import {
  runTriage,
  type TriageReportRpc,
  type TriageFileRpc,
} from "@/lib/tauri-ipc";
import { pickFolder } from "@/lib/file-picker";

interface Props {
  open: boolean;
  onClose: () => void;
}

const KIND_ICONS: Record<string, React.ReactNode> = {
  drone_image: <Radio className="h-3 w-3" />,
  las_pointcloud: <FileBox className="h-3 w-3" />,
  laz_pointcloud: <FileBox className="h-3 w-3" />,
  geotiff: <MapPin className="h-3 w-3" />,
  gnss_rinex: <Database className="h-3 w-3" />,
  gnss_nmea: <Database className="h-3 w-3" />,
  unknown: <FileText className="h-3 w-3" />,
};

const STATUS_COLORS: Record<string, string> = {
  ok: "#10B981",
  warning: "#F59E0B",
  error: "#EF4444",
  empty: "#6B7280",
};

export function TriageDialog({ open, onClose }: Props) {
  const [folderPath, setFolderPath] = useState("");
  const [running, setRunning] = useState(false);
  const [report, setReport] = useState<TriageReportRpc | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEscapeKey(onClose, open);

  // NOTE: useCallback MUST run before the early return below — React's
  // Rules of Hooks require hook call order to be identical on every render.
  // Returning early before a hook would crash the dialog the second time
  // it's opened (rendered fewer hooks than expected).
  const handleBrowse = useCallback(async () => {
    const p = await pickFolder("Select field data folder");
    if (p) setFolderPath(p);
  }, []);

  if (!open) return null;

  const handleRun = async () => {
    if (!folderPath) return;
    setRunning(true);
    setError(null);
    setReport(null);
    try {
      const r = await runTriage(folderPath);
      if (r) {
        setReport(r);
      } else {
        setError("Browser mode — triage analysis requires the native Tauri shell");
      }
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setRunning(false);
    }
  };

  const formatBytes = (bytes: number): string => {
    if (bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${(bytes / Math.pow(k, i)).toFixed(1)} ${sizes[i]}`;
  };

  const formatDuration = (secs: number): string => {
    if (secs < 60) return `${secs}s`;
    if (secs < 3600) return `${Math.floor(secs / 60)}m ${secs % 60}s`;
    return `${Math.floor(secs / 3600)}h ${Math.floor((secs % 3600) / 60)}m`;
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4"
      onClick={onClose}
    >
      <div
        onClick={(e) => e.stopPropagation()}
        className="flex max-h-[92vh] w-full max-w-4xl flex-col rounded-lg border border-navy-border bg-navy-panel shadow-2xl animate-scale-in"
      >
        {/* Header */}
        <div className="flex items-center justify-between border-b border-navy-border px-5 py-3">
          <h2 className="flex items-center gap-2 text-sm font-semibold text-white">
            <FolderOpen className="h-4 w-4" style={{ color: colors.industrialOrange }} />
            Mission Data Triage
          </h2>
          <button
            onClick={onClose}
            className="rounded p-1 text-steel-gray hover:bg-navy-elevated hover:text-white"
            aria-label="Close"
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        {/* Body */}
        <div className="flex-1 overflow-y-auto p-5">
          {error && (
            <div
              className="mb-4 rounded-md border p-3 text-xs flex items-start gap-2"
              style={{
                borderColor: `${colors.fail}40`,
                background: `${colors.fail}10`,
                color: colors.fail,
              }}
            >
              <AlertTriangle className="h-4 w-4 flex-shrink-0 mt-0.5" />
              <div className="flex-1">{error}</div>
            </div>
          )}

          {/* Folder picker */}
          <div className="mb-4">
            <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
              Field Data Folder
            </label>
            <div className="flex items-center gap-2">
              <button
                onClick={handleBrowse}
                disabled={running}
                className="flex items-center gap-1 rounded-md border border-navy-border bg-navy-base px-2.5 py-2 text-xs text-white hover:bg-navy-elevated disabled:opacity-50 flex-shrink-0"
              >
                <FolderOpen className="h-3.5 w-3.5" /> Browse
              </button>
              <input
                type="text"
                value={folderPath}
                onChange={(e) => setFolderPath(e.target.value)}
                placeholder="/path/to/field/data"
                disabled={running}
                className="flex-1 min-w-0 rounded-md border border-navy-border bg-navy-base px-3 py-2 font-mono text-xs text-white focus:outline-none disabled:opacity-50"
              />
              <button
                onClick={handleRun}
                disabled={!folderPath || running}
                className="flex items-center gap-1 rounded-md px-4 py-2 text-xs font-bold transition-colors disabled:opacity-40 flex-shrink-0"
                style={{ background: colors.industrialOrange, color: colors.navyBase }}
              >
                {running ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <FolderOpen className="h-3.5 w-3.5" />}
                {running ? "Scanning…" : "Run Triage"}
              </button>
            </div>
          </div>

          {/* Report */}
          {report && (
            <div className="space-y-4">
              {/* Summary tiles */}
              <div className="grid grid-cols-4 gap-2">
                <SummaryTile
                  icon={<FileText className="h-3.5 w-3.5" />}
                  label="Files"
                  value={report.total_files.toString()}
                  color={colors.white}
                />
                <SummaryTile
                  icon={<HardDrive className="h-3.5 w-3.5" />}
                  label="Total size"
                  value={formatBytes(report.total_size_bytes)}
                  color={colors.steelLight}
                />
                <SummaryTile
                  icon={<CheckCircle2 className="h-3.5 w-3.5" />}
                  label="Healthy"
                  value={report.healthy_files.toString()}
                  color={colors.pass}
                />
                <SummaryTile
                  icon={<AlertCircle className="h-3.5 w-3.5" />}
                  label="Issues"
                  value={(report.warning_files + report.error_files).toString()}
                  color={report.error_files > 0 ? colors.fail : colors.investigate}
                />
              </div>

              {/* CRS mismatch warning */}
              {report.crs_mismatch && (
                <div
                  className="rounded-md border p-3 text-xs"
                  style={{
                    borderColor: `${colors.fail}40`,
                    background: `${colors.fail}10`,
                    color: colors.fail,
                  }}
                >
                  <div className="flex items-center gap-2 font-semibold mb-1">
                    <AlertTriangle className="h-3.5 w-3.5" />
                    CRS Mismatch Detected
                  </div>
                  <div className="text-steel-light">
                    Multiple coordinate systems found: {report.detected_crs_list.join(", ")}
                  </div>
                </div>
              )}

              {/* Temporal span */}
              {report.time_span_secs !== null && report.time_span_secs > 0 && (
                <div
                  className="flex items-center gap-2 rounded-md border p-2 text-xs"
                  style={{ borderColor: colors.navyBorder, background: colors.navyBase }}
                >
                  <Clock className="h-3.5 w-3.5" style={{ color: colors.industrialOrange }} />
                  <span className="text-steel-light">
                    Acquisition time span: <span className="font-mono text-white">{formatDuration(report.time_span_secs)}</span>
                  </span>
                </div>
              )}

              {/* Warnings */}
              {report.warnings.length > 0 && (
                <div
                  className="rounded-md border p-3"
                  style={{
                    borderColor: `${colors.investigate}40`,
                    background: `${colors.investigate}10`,
                  }}
                >
                  <div className="mb-1 flex items-center gap-1.5 text-[10px] font-semibold uppercase tracking-wider" style={{ color: colors.investigate }}>
                    <AlertTriangle className="h-3 w-3" />
                    Warnings ({report.warnings.length})
                  </div>
                  <div className="space-y-0.5">
                    {report.warnings.slice(0, 10).map((w, i) => (
                      <div key={i} className="text-[10px] text-steel-light">• {w}</div>
                    ))}
                    {report.warnings.length > 10 && (
                      <div className="text-[10px] text-steel-gray">+ {report.warnings.length - 10} more…</div>
                    )}
                  </div>
                </div>
              )}

              {/* File list */}
              <div>
                <h3 className="mb-2 text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
                  File Inventory ({report.files.length})
                </h3>
                <div className="max-h-64 overflow-y-auto rounded-md border border-navy-border">
                  <table className="w-full text-[10px]">
                    <thead className="sticky top-0 bg-navy-panel">
                      <tr className="text-left text-steel-gray">
                        <th className="px-2 py-1.5 font-medium">Status</th>
                        <th className="px-2 py-1.5 font-medium">Type</th>
                        <th className="px-2 py-1.5 font-medium">Filename</th>
                        <th className="px-2 py-1.5 font-medium text-right">Size</th>
                        <th className="px-2 py-1.5 font-medium text-right">Points</th>
                        <th className="px-2 py-1.5 font-medium">Bounds</th>
                      </tr>
                    </thead>
                    <tbody>
                      {report.files.map((f, i) => (
                        <FileRow key={i} file={f} formatBytes={formatBytes} />
                      ))}
                    </tbody>
                  </table>
                </div>
              </div>
            </div>
          )}

          {/* Empty state */}
          {!report && !running && (
            <div
              className="rounded-md border border-dashed p-8 text-center"
              style={{ borderColor: colors.navyBorder, background: colors.navyBase }}
            >
              <FolderOpen className="mx-auto mb-3 h-8 w-8" style={{ color: colors.steelGray }} />
              <div className="text-xs text-steel-light">
                Select a folder and click <span style={{ color: colors.industrialOrange }}>Run Triage</span> to scan for:
              </div>
              <div className="mt-2 grid grid-cols-2 gap-1 text-[10px] text-steel-gray max-w-md mx-auto">
                <div>• Drone image EXIF (GPS + timestamps)</div>
                <div>• LAS/LAZ header bounds + point counts</div>
                <div>• RINEX GNSS approximate positions</div>
                <div>• NMEA trajectory bounds</div>
                <div>• CRS mismatches across files</div>
                <div>• Empty/corrupt file detection</div>
              </div>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between border-t border-navy-border px-5 py-2 text-[10px] text-steel-gray">
          <div>Prevents costly return trips — verify coverage before leaving the field.</div>
          <div>v0.1.0</div>
        </div>
      </div>
    </div>
  );
}

function SummaryTile({
  icon, label, value, color,
}: {
  icon: React.ReactNode; label: string; value: string; color: string;
}) {
  return (
    <div
      className="rounded-md border p-2.5"
      style={{ borderColor: colors.navyBorder, background: colors.navyBase }}
    >
      <div className="flex items-center gap-1 text-[9px] uppercase tracking-wider" style={{ color }}>
        {icon}
        {label}
      </div>
      <div className="mt-1 font-mono text-sm font-bold tabular-nums text-white">{value}</div>
    </div>
  );
}

function FileRow({
  file, formatBytes,
}: {
  file: TriageFileRpc; formatBytes: (b: number) => string;
}) {
  const statusColor = STATUS_COLORS[file.status] || colors.steelGray;
  return (
    <tr className="border-t border-navy-border hover:bg-navy-elevated/30">
      <td className="px-2 py-1.5">
        <span
          className="inline-block h-2 w-2 rounded-full"
          style={{ background: statusColor }}
          title={file.status}
        />
      </td>
      <td className="px-2 py-1.5" style={{ color: colors.steelLight }}>
        {KIND_ICONS[file.kind] || <FileText className="h-3 w-3" />}
      </td>
      <td className="px-2 py-1.5 truncate text-white max-w-[200px]" title={file.filename}>
        {file.filename}
      </td>
      <td className="px-2 py-1.5 text-right font-mono text-steel-light">
        {formatBytes(file.size_bytes)}
      </td>
      <td className="px-2 py-1.5 text-right font-mono text-steel-light">
        {file.point_count ? file.point_count.toLocaleString() : "—"}
      </td>
      <td className="px-2 py-1.5 font-mono text-[9px] text-steel-gray">
        {file.bounds
          ? `${file.bounds[0].toFixed(4)}, ${file.bounds[1].toFixed(4)}`
          : "—"}
      </td>
    </tr>
  );
}
