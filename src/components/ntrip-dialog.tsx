/**
 * NTRIP Client Dialog — RTK correction streaming configuration.
 *
 * Connects to an NTRIP caster to receive RTCM v3 correction data for
 * real-time kinematic (RTK) positioning. This eliminates the need for
 * a separate NTRIP client app — corrections stream directly into MetaRDU.
 */

import { useState, useEffect } from "react";
import { useEscapeKey } from "@/lib/use-escape-key";
import {
  X, Radio, Loader2, Wifi, WifiOff,
} from "lucide-react";
import { colors } from "@/lib/tokens";
import {
  startNtrip, stopNtrip, getNtripStatus,
  type NtripConfigRpc, type NtripStatusRpc,
} from "@/lib/tauri-ipc";

interface Props {
  open: boolean;
  onClose: () => void;
}

export function NtripDialog({ open, onClose }: Props) {
  const [host, setHost] = useState("ntrip.embassy-data.com");
  const [port, setPort] = useState(2101);
  const [mountpoint, setMountpoint] = useState("RTCM3GG");
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [connecting, setConnecting] = useState(false);
  const [status, setStatus] = useState<NtripStatusRpc | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEscapeKey(onClose, open);
  if (!open) return null;

  // Poll status when connected
  useEffect(() => {
    if (!open) return;
    getNtripStatus().then(setStatus).catch(() => {});
    const interval = setInterval(() => {
      getNtripStatus().then(setStatus).catch(() => {});
    }, 2000);
    return () => clearInterval(interval);
  }, [open]);

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
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4"
      onClick={onClose}
    >
      <div
        onClick={(e) => e.stopPropagation()}
        className="flex max-h-[88vh] w-full max-w-lg flex-col rounded-lg border border-navy-border bg-navy-panel shadow-2xl animate-scale-in"
      >
        {/* Header */}
        <div className="flex items-center justify-between border-b border-navy-border px-5 py-3">
          <h2 className="flex items-center gap-2 text-sm font-semibold text-white">
            <Radio className="h-4 w-4" style={{ color: colors.marineTurquoise }} />
            NTRIP Client
            {isConnected && (
              <span
                className="flex items-center gap-1 rounded-sm px-1.5 py-0.5 text-[9px] font-semibold uppercase"
                style={{ background: `${colors.pass}20`, color: colors.pass }}
              >
                <span className="h-1.5 w-1.5 rounded-full animate-pulse" style={{ background: colors.pass }} />
                Connected
              </span>
            )}
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
        <div className="flex-1 overflow-y-auto p-5 space-y-4">
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
                ) : (
                  <WifiOff className="h-4 w-4" style={{ color: colors.fail }} />
                )}
                <span className="text-xs font-semibold text-white">
                  {isConnected ? "Streaming RTCM corrections" : "Disconnected"}
                </span>
              </div>
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
                  className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 font-mono text-xs text-white focus:outline-none disabled:opacity-50"
                />
              </div>
              <input
                type="number"
                value={port}
                onChange={(e) => setPort(parseInt(e.target.value) || 2101)}
                disabled={isConnected || connecting}
                className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 font-mono text-xs text-white focus:outline-none disabled:opacity-50"
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
              className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 font-mono text-xs text-white focus:outline-none disabled:opacity-50"
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
                className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 text-xs text-white focus:outline-none disabled:opacity-50"
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
                className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 text-xs text-white focus:outline-none disabled:opacity-50"
              />
            </div>
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
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between border-t border-navy-border px-5 py-2 text-[10px] text-steel-gray">
          <div>RTCM v3 corrections — eliminates the need for a separate NTRIP client.</div>
        </div>
      </div>
    </div>
  );
}
