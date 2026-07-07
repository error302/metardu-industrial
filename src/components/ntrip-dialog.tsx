/**
 * NTRIP Client Dialog — RTK correction streaming configuration.
 *
 * Connects to an NTRIP caster to receive RTCM v3 correction data for
 * real-time kinematic (RTK) positioning. This eliminates the need for
 * a separate NTRIP client app — corrections stream directly into MetaRDU.
 */

import { useState, useEffect } from "react";
import {
  Radio, Loader2, Wifi, WifiOff, AlertTriangle, RefreshCw,
} from "lucide-react";
import { colors } from "@/lib/tokens";
import { DialogShell, DialogButton } from "@/components/dialog-shell";
import {
  startNtrip, stopNtrip, getNtripStatus,
  type NtripConfigRpc, type NtripStatusRpc,
} from "@/lib/tauri-ipc";

interface Props {
  open: boolean;
  onClose: () => void;
}

/** Compute a human-readable correction age + staleness state.
 *
 *  Field crews care about ONE number more than any other: how old is my
 *  RTK correction? Anything older than ~10s means the fix is degraded;
 *  older than ~30s means it's effectively lost. We compute this client-side
 *  from `last_message_epoch_ms` and the wall clock so it ticks live every
 *  second instead of waiting for the 2s IPC poll.
 */
function describeCorrectionAge(
  status: NtripStatusRpc | null,
  nowMs: number,
): { text: string; state: "fresh" | "aging" | "stale" | "none"; secs: number | null } {
  if (!status?.last_message_epoch_ms) {
    return { text: "—", state: "none", secs: null };
  }
  const secs = Math.max(0, Math.floor((nowMs - status.last_message_epoch_ms) / 1000));
  // Thresholds per common field practice:
  //   < 5s: fresh (green) — RTK fix is solid
  //   5–15s: aging (yellow) — fix may degrade, watch it
  //   > 15s: stale (red) — fix is degraded or lost
  const state = secs < 5 ? "fresh" : secs <= 15 ? "aging" : "stale";
  const text = secs < 60 ? `${secs}s ago` : `${Math.floor(secs / 60)}m ${secs % 60}s ago`;
  return { text, state, secs };
}

export function NtripDialog({ open, onClose }: Props) {
  const [host, setHost] = useState("ntrip.embassy-data.com");
  const [port, setPort] = useState(2101);
  const [mountpoint, setMountpoint] = useState("RTCM3GG");
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [useTls, setUseTls] = useState(false);
  const [connecting, setConnecting] = useState(false);
  const [status, setStatus] = useState<NtripStatusRpc | null>(null);
  const [error, setError] = useState<string | null>(null);
  // Force a 1s re-render so the correction-age clock ticks even between
  // the 2s IPC polls. The poll gives us `last_message_epoch_ms`; this
  // tick is what makes the displayed age actually count up live.
  const [, setTick] = useState(0);


  // All hooks MUST be before the early return — React rules of hooks.
  useEffect(() => {
    if (!open) return;
    getNtripStatus().then(setStatus).catch(() => {});
    const poll = setInterval(() => {
      getNtripStatus().then(setStatus).catch(() => {});
    }, 2000);
    const ticker = setInterval(() => setTick((t) => t + 1), 1000);
    return () => {
      clearInterval(poll);
      clearInterval(ticker);
    };
  }, [open]);


  const correctionAge = describeCorrectionAge(status, Date.now());

  const handleConnect = async () => {
    setConnecting(true);
    setError(null);
    try {
      const config: NtripConfigRpc = {
        host,
        port,
        mountpoint,
        username: username || null,
        password: password || null,
        timeout_secs: 10,
        use_tls: useTls,
      };
      const s = await startNtrip(config);
      if (s) {
        setStatus(s);
      } else {
        setError("Browser mode — NTRIP requires the native Tauri shell");
      }
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setConnecting(false);
    }
  };

  const handleDisconnect = async () => {
    await stopNtrip();
    setStatus(null);
  };

  const isConnected = status?.connected ?? false;

return (
    <DialogShell
      open={open}
      onClose={onClose}
      title="NTRIP Client"
      icon={<Radio className="h-4 w-4" />}
      iconColor={colors.marineTurquoise}
      maxWidth="max-w-2xl"
      subtitle="RTCM3 correction stream"
      footerHint="TCP + TLS + base64 auth"
      actions={
        <DialogButton variant="secondary" onClick={onClose}>Close</DialogButton>
      }
    >
          {error && (
            <div
              className="rounded-md border p-3 text-xs"
              style={{ borderColor: `${colors.fail}40`, background: `${colors.fail}10`, color: colors.fail }}
            >
              {error}
            </div>
          )}

          {/* Connection status */}
          {status && (
            <div
              className="rounded-md border p-3"
              style={{
                borderColor: isConnected ? `${colors.pass}40` : `${colors.fail}40`,
                background: isConnected ? `${colors.pass}08` : `${colors.fail}08`,
              }}
            >
              <div className="flex items-center gap-2 mb-2">
                {isConnected ? (
                  <Wifi className="h-4 w-4" style={{ color: colors.pass }} />
                ) : status.reconnecting ? (
                  <RefreshCw className="h-4 w-4 animate-spin" style={{ color: colors.industrialOrange }} />
                ) : (
                  <WifiOff className="h-4 w-4" style={{ color: colors.fail }} />
                )}
                <span className="text-xs font-semibold text-white">
                  {isConnected
                    ? "Streaming RTCM corrections"
                    : status.reconnecting
                      ? `Reconnecting… (attempt #${status.reconnect_attempts})`
                      : "Disconnected"}
                </span>
              </div>

              {/* ─── Correction Age — the #1 field-crew metric ─── */}
              {isConnected && (
                <div
                  className="mb-3 rounded-md border p-2.5"
                  style={{
                    borderColor:
                      correctionAge.state === "fresh"
                        ? `${colors.pass}40`
                        : correctionAge.state === "aging"
                          ? `${colors.industrialOrange}60`
                          : `${colors.fail}60`,
                    background:
                      correctionAge.state === "fresh"
                        ? `${colors.pass}08`
                        : correctionAge.state === "aging"
                          ? `${colors.industrialOrange}08`
                          : `${colors.fail}08`,
                  }}
                >
                  <div className="flex items-center justify-between">
                    <div>
                      <div className="text-[9px] font-semibold uppercase tracking-wider text-steel-gray">
                        Correction Age
                      </div>
                      <div
                        className="font-mono text-base font-bold tabular-nums"
                        style={{
                          color:
                            correctionAge.state === "fresh"
                              ? colors.pass
                              : correctionAge.state === "aging"
                                ? colors.industrialOrange
                                : colors.fail,
                        }}
                      >
                        {correctionAge.text}
                      </div>
                    </div>
                    <div className="text-right text-[9px] leading-tight text-steel-gray">
                      {correctionAge.state === "fresh" && "RTK fix solid"}
                      {correctionAge.state === "aging" && "Fix may degrade"}
                      {correctionAge.state === "stale" && "Fix degraded or lost"}
                      {correctionAge.state === "none" && "No messages yet"}
                    </div>
                  </div>
                  {correctionAge.state !== "fresh" && correctionAge.state !== "none" && (
                    <div className="mt-1.5 flex items-center gap-1 text-[10px]" style={{ color: correctionAge.state === "stale" ? colors.fail : colors.industrialOrange }}>
                      <AlertTriangle className="h-3 w-3" />
                      {correctionAge.state === "stale"
                        ? "Check cell signal — corrections >15s old"
                        : "Corrections >5s old — monitor fix quality"}
                    </div>
                  )}
                </div>
              )}

              {isConnected && (
                <div className="grid grid-cols-3 gap-2 text-[10px]">
                  <div>
                    <div className="text-steel-gray">Messages</div>
                    <div className="font-mono text-white tabular-nums">{status.messages_received.toLocaleString()}</div>
                  </div>
                  <div>
                    <div className="text-steel-gray">Data received</div>
                    <div className="font-mono text-white tabular-nums">{(status.bytes_received / 1024).toFixed(1)} KB</div>
                  </div>
                  <div>
                    <div className="text-steel-gray">Last msg type</div>
                    <div className="font-mono text-white">{status.last_message_type ?? "—"}</div>
                  </div>
                  <div>
                    <div className="text-steel-gray">Uptime</div>
                    <div className="font-mono text-white">{Math.floor(status.uptime_secs / 60)}m {status.uptime_secs % 60}s</div>
                  </div>
                  <div>
                    <div className="text-steel-gray">Mountpoint</div>
                    <div className="font-mono text-white truncate">{status.mountpoint}</div>
                  </div>
                </div>
              )}
              {status.last_error && (
                <div className="mt-2 text-[10px]" style={{ color: colors.fail }}>
                  ⚠ {status.last_error}
                </div>
              )}
            </div>
          )}

          {/* Caster config */}
          <div>
            <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
              NTRIP Caster
            </label>
            <div className="grid grid-cols-3 gap-2">
              <div className="col-span-2">
                <input
                  type="text"
                  value={host}
                  onChange={(e) => setHost(e.target.value)}
                  placeholder="ntrip.example.com"
                  disabled={isConnected || connecting}
                  className="input-enterprise w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 font-mono text-xs text-white focus:outline-none disabled:opacity-50"
                />
              </div>
              <input
                type="number"
                value={port}
                onChange={(e) => setPort(parseInt(e.target.value) || 2101)}
                disabled={isConnected || connecting}
                className="input-enterprise w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 font-mono text-xs text-white focus:outline-none disabled:opacity-50"
              />
            </div>
          </div>

          {/* Mountpoint */}
          <div>
            <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
              Mountpoint
            </label>
            <input
              type="text"
              value={mountpoint}
              onChange={(e) => setMountpoint(e.target.value)}
              placeholder="RTCM3GG"
              disabled={isConnected || connecting}
              className="input-enterprise w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 font-mono text-xs text-white focus:outline-none disabled:opacity-50"
            />
          </div>

          {/* Auth (optional) */}
          <div className="grid grid-cols-2 gap-2">
            <div>
              <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
                Username (optional)
              </label>
              <input
                type="text"
                value={username}
                onChange={(e) => setUsername(e.target.value)}
                placeholder="user"
                disabled={isConnected || connecting}
                className="input-enterprise w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 text-xs text-white focus:outline-none disabled:opacity-50"
              />
            </div>
            <div>
              <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
                Password
              </label>
              <input
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                placeholder="••••"
                disabled={isConnected || connecting}
                className="input-enterprise w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 text-xs text-white focus:outline-none disabled:opacity-50"
              />
            </div>
          </div>

          {/* TLS toggle */}
          <div className="flex items-center gap-2 rounded-md border border-navy-border bg-navy-base px-3 py-2">
            <input
              type="checkbox"
              id="useTls"
              checked={useTls}
              onChange={(e) => setUseTls(e.target.checked)}
              disabled={isConnected || connecting}
              className="h-3.5 w-3.5"
            />
            <label htmlFor="useTls" className="text-xs text-white cursor-pointer select-none">
              Use TLS (ntrips://)
            </label>
            <span className="text-[9px] text-steel-gray ml-auto">
              {useTls ? "Encrypted" : "Unencrypted — use only on trusted networks"}
            </span>
          </div>

          {/* Connect/Disconnect button */}
          <button
            onClick={isConnected ? handleDisconnect : handleConnect}
            disabled={connecting || (!isConnected && !host)}
            className="flex w-full items-center justify-center gap-2 rounded-md px-4 py-2.5 text-sm font-bold transition-colors disabled:opacity-40"
            style={{
              background: isConnected ? colors.fail : colors.marineTurquoise,
              color: colors.navyBase,
            }}
          >
            {connecting ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : isConnected ? (
              <WifiOff className="h-4 w-4" />
            ) : (
              <Wifi className="h-4 w-4" />
            )}
            {connecting ? "Connecting…" : isConnected ? "Disconnect" : "Connect"}
          </button>
    </DialogShell>
  );
}
