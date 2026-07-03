/**
 * Telemetry Dialog — Sprint 7.
 *
 * Opt-in telemetry + crash reporter settings. Shows:
 *   - Opt-in toggle for usage stats
 *   - Opt-in toggle for crash auto-submit
 *   - Recent events (for transparency)
 *   - Pending crash dumps (user can review before submitting)
 *   - Aggregated stats (top commands, avg IPC duration, uptime)
 *
 * Privacy model: OFF by default, only sends app version + OS + command
 * name + error message + license tier. NEVER sends file paths or data.
 */

import { useState, useEffect } from "react";
import {
  X, Activity, AlertTriangle, Send, CheckCircle2, RefreshCw, BarChart3, Clock,
} from "lucide-react";
import { colors } from "@/lib/tokens";
import {
  getTelemetryConfig,
  updateTelemetryConfig,
  getTelemetryStats,
  getPendingCrashes,
  getRecentEvents,
  markCrashSubmitted,
  type TelemetryConfig,
  type TelemetryStats,
  type CrashDump,
  type TelemetryEvent,
} from "@/lib/tauri-ipc";

interface Props {
  open: boolean;
  onClose: () => void;
}

export function TelemetryDialog({ open, onClose }: Props) {
  const [config, setConfig] = useState<TelemetryConfig | null>(null);
  const [stats, setStats] = useState<TelemetryStats | null>(null);
  const [crashes, setCrashes] = useState<CrashDump[]>([]);
  const [events, setEvents] = useState<TelemetryEvent[]>([]);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (open) {
      refresh();
    }
  }, [open]);

  if (!open) return null;

  async function refresh() {
    try {
      const [cfg, s, c, e] = await Promise.all([
        getTelemetryConfig(),
        getTelemetryStats(),
        getPendingCrashes(),
        getRecentEvents(20),
      ]);
      if (cfg) setConfig(cfg);
      if (s) setStats(s);
      setCrashes(c);
      setEvents(e);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  async function handleToggle(field: "enabled" | "crash_auto_submit") {
    if (!config) return;
    setSaving(true);
    try {
      const updated = { ...config, [field]: !config[field] };
      await updateTelemetryConfig(updated);
      setConfig(updated);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setSaving(false);
    }
  }

  async function handleSubmitCrash(crashId: string) {
    try {
      await markCrashSubmitted(crashId);
      setCrashes(crashes.filter((c) => c.crash_id !== crashId));
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
      onClick={onClose}
    >
      <div
        onClick={(e) => e.stopPropagation()}
        className="flex max-h-[88vh] w-full max-w-3xl flex-col rounded-lg border border-navy-border bg-navy-panel shadow-2xl"
      >
        {/* Header */}
        <div className="flex items-center justify-between border-b border-navy-border px-5 py-3">
          <h2 className="flex items-center gap-2 text-sm font-semibold text-white">
            <Activity className="h-4 w-4" style={{ color: colors.marineTurquoise }} />
            Telemetry & Crash Reporter
          </h2>
          <div className="flex items-center gap-2">
            <button
              onClick={refresh}
              className="rounded p-1 text-steel-gray hover:bg-navy-elevated hover:text-white"
              title="Refresh"
            >
              <RefreshCw className="h-3.5 w-3.5" />
            </button>
            <button onClick={onClose} className="rounded p-1 text-steel-gray hover:bg-navy-elevated hover:text-white">
              <X className="h-4 w-4" />
            </button>
          </div>
        </div>

        {/* Body */}
        <div className="flex-1 overflow-y-auto p-5 space-y-4">
          {error && (
            <div className="rounded-md border p-3 text-xs"
              style={{ borderColor: `${colors.fail}40`, background: `${colors.fail}10`, color: colors.fail }}>
              {error}
            </div>
          )}

          {/* Privacy notice */}
          <div className="rounded-md border p-3 text-xs"
            style={{ borderColor: `${colors.marineTurquoise}40`, background: `${colors.marineTurquoise}10` }}>
            <div className="font-semibold" style={{ color: colors.marineTurquoise }}>Privacy</div>
            <div className="mt-1 text-steel-light">
              Telemetry is OFF by default. If enabled, we collect: app version, OS, command name,
              error message, license tier. We NEVER collect: file paths, customer data, point cloud
              data, or coordinates. Crash dumps are stored locally first — you review before submitting.
            </div>
          </div>

          {/* Opt-in toggles */}
          {config && (
            <div className="space-y-2">
              <ToggleRow
                label="Usage Telemetry"
                description="Anonymous usage stats help us prioritize features"
                enabled={config.enabled}
                onToggle={() => handleToggle("enabled")}
                disabled={saving}
              />
              <ToggleRow
                label="Crash Auto-Submit"
                description="Automatically submit crash dumps (otherwise manual review)"
                enabled={config.crash_auto_submit}
                onToggle={() => handleToggle("crash_auto_submit")}
                disabled={saving}
              />
              <div className="rounded-md border border-navy-border bg-navy-base p-2 text-[10px] text-steel-gray">
                Anonymous ID: <span className="font-mono">{config.anonymous_id.slice(0, 24)}…</span>
                <br />
                Endpoint: <span className="font-mono">{config.endpoint_url || "(local only — no remote submission)"}</span>
              </div>
            </div>
          )}

          {/* Stats */}
          {stats && (
            <div>
              <h4 className="mb-2 flex items-center gap-1 text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
                <BarChart3 className="h-3 w-3" /> Session Stats
              </h4>
              <div className="grid grid-cols-4 gap-2 text-xs">
                <StatTile label="Total Events" value={stats.total_events.toLocaleString()} />
                <StatTile label="Total Crashes" value={stats.total_crashes.toLocaleString()} />
                <StatTile label="Pending Crashes" value={stats.pending_crashes.toLocaleString()} />
                <StatTile label="Uptime" value={formatUptime(stats.uptime_seconds)} />
                <StatTile label="Avg IPC (ms)" value={stats.avg_ipc_duration_ms.toFixed(1)} />
                <StatTile label="Top Command" value={stats.top_commands[0]?.[0] ?? "—"} />
                <StatTile label="Top Cmd Count" value={(stats.top_commands[0]?.[1] ?? 0).toLocaleString()} />
                <StatTile label="Top Failure" value={stats.top_failures[0]?.[0] ?? "—"} />
              </div>
            </div>
          )}

          {/* Pending crashes */}
          {crashes.length > 0 && (
            <div>
              <h4 className="mb-2 flex items-center gap-1 text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
                <AlertTriangle className="h-3 w-3" style={{ color: colors.fail }} />
                Pending Crash Dumps ({crashes.length})
              </h4>
              <div className="space-y-2">
                {crashes.map((crash) => (
                  <div key={crash.crash_id} className="rounded-md border p-2 text-[10px]"
                    style={{ borderColor: `${colors.fail}40`, background: `${colors.fail}10` }}>
                    <div className="flex items-start justify-between">
                      <div className="flex-1 min-w-0">
                        <div className="font-mono text-white">{crash.command}</div>
                        <div className="text-steel-light">{crash.message}</div>
                        <div className="text-steel-gray">
                          {new Date(crash.timestamp_ms).toLocaleString()} · v{crash.app_version} · {crash.os_info}
                        </div>
                      </div>
                      <button
                        onClick={() => handleSubmitCrash(crash.crash_id)}
                        className="ml-2 flex items-center gap-1 rounded px-2 py-1 text-[10px]"
                        style={{ background: colors.industrialOrange, color: colors.navyBase }}
                      >
                        <Send className="h-3 w-3" /> Submit
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Recent events */}
          {events.length > 0 && (
            <div>
              <h4 className="mb-2 flex items-center gap-1 text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
                <Clock className="h-3 w-3" /> Recent Events (last 20)
              </h4>
              <div className="max-h-32 overflow-y-auto rounded-md border border-navy-border">
                <table className="w-full text-left text-[10px]">
                  <thead className="sticky top-0 bg-navy-panel text-steel-gray">
                    <tr>
                      <th className="px-2 py-1">Time</th>
                      <th className="px-2 py-1">Type</th>
                      <th className="px-2 py-1">Name</th>
                      <th className="px-2 py-1 text-right">Duration</th>
                      <th className="px-2 py-1">Status</th>
                    </tr>
                  </thead>
                  <tbody>
                    {events.map((e, i) => (
                      <tr key={i} className="border-t border-navy-border">
                        <td className="px-2 py-1 font-mono text-steel-gray">
                          {new Date(e.timestamp_ms).toLocaleTimeString()}
                        </td>
                        <td className="px-2 py-1 text-steel-light">{e.event_type}</td>
                        <td className="px-2 py-1 font-mono text-white">{e.event_name}</td>
                        <td className="px-2 py-1 text-right font-mono text-steel-light">
                          {e.duration_ms !== null ? `${e.duration_ms}ms` : "—"}
                        </td>
                        <td className="px-2 py-1" style={{ color: e.success ? colors.pass : colors.fail }}>
                          ● {e.success ? "OK" : "FAIL"}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between border-t border-navy-border px-5 py-3 text-[10px] text-steel-gray">
          <span>Your privacy is protected — no data leaves your machine without consent.</span>
          <button
            onClick={onClose}
            className="rounded-md px-3 py-1 text-xs font-medium"
            style={{ background: colors.pass, color: colors.navyBase }}
          >
            Close
          </button>
        </div>
      </div>
    </div>
  );
}

function ToggleRow({
  label, description, enabled, onToggle, disabled,
}: {
  label: string; description: string; enabled: boolean;
  onToggle: () => void; disabled: boolean;
}) {
  return (
    <div className="flex items-center justify-between rounded-md border border-navy-border bg-navy-base p-3">
      <div className="flex-1">
        <div className="text-sm font-medium text-white">{label}</div>
        <div className="text-[10px] text-steel-gray">{description}</div>
      </div>
      <button
        onClick={onToggle}
        disabled={disabled}
        className="relative h-6 w-11 rounded-full transition-colors disabled:opacity-40"
        style={{ background: enabled ? colors.pass : colors.steelGray }}
      >
        <div
          className="absolute top-0.5 h-5 w-5 rounded-full bg-white transition-transform"
          style={{ transform: enabled ? "translateX(22px)" : "translateX(2px)" }}
        />
      </button>
    </div>
  );
}

function StatTile({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-md border border-navy-border bg-navy-base p-2">
      <div className="text-[9px] uppercase tracking-wider text-steel-gray">{label}</div>
      <div className="mt-0.5 truncate font-mono text-sm font-bold text-white" title={value}>{value}</div>
    </div>
  );
}

function formatUptime(secs: u64_placeholder): string {
  if (secs < 60) return `${secs}s`;
  if (secs < 3600) return `${Math.floor(secs / 60)}m ${secs % 60}s`;
  return `${Math.floor(secs / 3600)}h ${Math.floor((secs % 3600) / 60)}m`;
}

// Type alias for readability
type u64_placeholder = number;
