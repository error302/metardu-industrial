/**
 * RTK Rover Stream — Sprint 11 Real-Time #1.
 *
 * Connects to a TCP NMEA source (serial-to-TCP bridge or base station),
 * streams GGA/RMC/GLL/GSA/VTG sentences, and displays:
 *   - Latest position (lat/lon/alt/fix quality/sats/HDOP)
 *   - Connection status + sentence counters
 *   - Live position trail (60-second SVG sparkline of lat/lon drift)
 *
 * The position is also rendered on the OpenLayers map via a
 * RoverPositionOverlay (added separately in workspace-shell).
 *
 * Workflow:
 *   1. Enter TCP host + port (e.g., 127.0.0.1:8500 for com2tcp)
 *   2. Click Connect → start_rover_stream_cmd
 *   3. Position + trail update at 5 Hz via get_rover_position_cmd
 *   4. Click Disconnect to stop
 */

import { useState, useEffect, useRef } from "react";
import { X, Satellite, Loader2, Play, Square } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { colors } from "@/lib/tokens";
import { isNative } from "@/lib/tauri-ipc";
import { useEscapeKey } from "@/lib/use-escape-key";

interface RoverPosition {
  timestamp: number;
  latitude: number | null;
  longitude: number | null;
  altitude_m: number | null;
  fix_quality: number | null;
  satellites: number | null;
  hdop: number | null;
  speed_mps: number | null;
  course_deg: number | null;
  age_of_diff_s: number | null;
  diff_station_id: number | null;
}

interface RoverStatus {
  connected: boolean;
  is_running: boolean;
  last_error: string | null;
  sentences_parsed: number;
  sentences_rejected: number;
}

interface Props {
  open: boolean;
  onClose: () => void;
}

const FIX_LABELS: Record<number, { label: string; color: string }> = {
  0: { label: "No Fix", color: colors.fail },
  1: { label: "GPS", color: colors.warn },
  2: { label: "DGPS", color: colors.info },
  3: { label: "PPS", color: colors.info },
  4: { label: "RTK Fixed", color: colors.pass },
  5: { label: "RTK Float", color: colors.pass },
  6: { label: "Dead Reckoning", color: colors.warn },
  7: { label: "Manual", color: colors.steelLight },
  8: { label: "Simulation", color: colors.steelLight },
};

export function RoverStreamDialog({ open, onClose }: Props) {
  const [host, setHost] = useState("127.0.0.1");
  const [port, setPort] = useState("8500");
  const [position, setPosition] = useState<RoverPosition | null>(null);
  const [status, setStatus] = useState<RoverStatus | null>(null);
  const [trail, setTrail] = useState<RoverPosition[]>([]);
  const [connecting, setConnecting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null);

  useEscapeKey(onClose, open);
  if (!open) return null;

  // Poll for updates at 5 Hz when running
  useEffect(() => {
    if (!open) return;
    const poll = async () => {
      if (!isNative()) return;
      try {
        const [pos, stat, tr] = await Promise.all([
          invoke<RoverPosition>("get_rover_position_cmd"),
          invoke<RoverStatus>("get_rover_status_cmd"),
          invoke<RoverPosition[]>("get_rover_trail_cmd"),
        ]);
        setPosition(pos);
        setStatus(stat);
        setTrail(tr);
      } catch {
        // ignore — probably not running yet
      }
    };
    pollRef.current = setInterval(poll, 200);
    poll();
    return () => {
      if (pollRef.current) clearInterval(pollRef.current);
    };
  }, [open]);

  async function handleConnect() {
    setConnecting(true);
    setError(null);
    try {
      if (!isNative()) {
        setError("Browser mode — rover stream requires the native Tauri shell");
        return;
      }
      const portNum = parseInt(port);
      if (Number.isNaN(portNum) || portNum < 1 || portNum > 65535) {
        throw new Error("Invalid port — must be 1-65535");
      }
      await invoke("start_rover_stream_cmd", { host, port: portNum });
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setConnecting(false);
    }
  }

  async function handleDisconnect() {
    try {
      await invoke("stop_rover_stream_cmd");
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  const isRunning = status?.is_running ?? false;
  const fix = position?.fix_quality != null ? FIX_LABELS[position.fix_quality] ?? { label: "Unknown", color: colors.steelLight } : null;

  // SVG trail sparkline (lat vs lon)
  const W = 280, H = 180, pad = 20;
  const trailPoints = trail.filter(p => p.latitude != null && p.longitude != null);
  const bounds = trailPoints.length >= 2 ? {
    minLat: Math.min(...trailPoints.map(p => p.latitude!)),
    maxLat: Math.max(...trailPoints.map(p => p.latitude!)),
    minLon: Math.min(...trailPoints.map(p => p.longitude!)),
    maxLon: Math.max(...trailPoints.map(p => p.longitude!)),
  } : null;
  const lonRange = bounds ? Math.max(0.00001, bounds.maxLon - bounds.minLon) : 1;
  const latRange = bounds ? Math.max(0.00001, bounds.maxLat - bounds.minLat) : 1;

  function lonToX(lon: number): number {
    if (!bounds) return W / 2;
    return pad + ((lon - bounds.minLon) / lonRange) * (W - 2 * pad);
  }
  function latToY(lat: number): number {
    if (!bounds) return H / 2;
    return H - pad - ((lat - bounds.minLat) / latRange) * (H - 2 * pad);
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
        <div className="flex items-center justify-between border-b border-navy-border px-5 py-3">
          <h2 className="flex items-center gap-2 text-sm font-semibold text-white">
            <Satellite className="h-4 w-4" style={{ color: colors.marine }} />
            RTK Rover Stream
          </h2>
          <button onClick={onClose} className="rounded p-1 text-steel-gray hover:bg-navy-elevated hover:text-white">
            <X className="h-4 w-4" />
          </button>
        </div>

        <div className="flex-1 overflow-y-auto p-5 space-y-4">
          {/* Connection controls */}
          <div className="grid grid-cols-[1fr_120px_auto] items-end gap-3">
            <div>
              <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">TCP Host</label>
              <input
                type="text"
                value={host}
                onChange={(e) => setHost(e.target.value)}
                disabled={isRunning}
                placeholder="127.0.0.1"
                className="w-full rounded-md border border-navy-border bg-navy-base px-2 py-1.5 font-mono text-sm text-white focus:border-marine focus:outline-none disabled:opacity-50"
              />
            </div>
            <div>
              <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">Port</label>
              <input
                type="number"
                value={port}
                onChange={(e) => setPort(e.target.value)}
                disabled={isRunning}
                min={1}
                max={65535}
                className="w-full rounded-md border border-navy-border bg-navy-base px-2 py-1.5 font-mono text-sm text-white focus:border-marine focus:outline-none disabled:opacity-50"
              />
            </div>
            {isRunning ? (
              <button
                onClick={handleDisconnect}
                className="flex items-center gap-1.5 rounded-md px-4 py-1.5 text-xs font-medium"
                style={{ background: colors.fail, color: colors.white }}
              >
                <Square className="h-3 w-3" /> Disconnect
              </button>
            ) : (
              <button
                onClick={handleConnect}
                disabled={connecting}
                className="flex items-center gap-1.5 rounded-md px-4 py-1.5 text-xs font-medium disabled:opacity-40"
                style={{ background: colors.marine, color: colors.navyBase }}
              >
                {connecting ? <Loader2 className="h-3 w-3 animate-spin" /> : <Play className="h-3 w-3" />}
                {connecting ? "Connecting…" : "Connect"}
              </button>
            )}
          </div>

          <p className="rounded-md bg-navy-base p-2 text-[10px] leading-relaxed text-steel-gray">
            <strong className="text-steel-light">Setup:</strong> Connect your GNSS receiver's serial port to a
            TCP bridge (e.g., <span className="font-mono">com2tcp</span> on Windows). Typical receivers output
            NMEA sentences at 1 Hz or 5 Hz. For RTK correction, pair with the NTRIP Client (Sprint 9) — corrections
            are sent to the receiver, which then outputs corrected GGA sentences with fix quality = 4 (RTK Fixed).
          </p>

          {error && (
            <div className="rounded-md border p-3 text-xs" style={{ borderColor: `${colors.fail}40`, background: `${colors.fail}10`, color: colors.fail }}>
              {error}
            </div>
          )}

          {/* Status row */}
          {status && (
            <div className="grid grid-cols-4 gap-2">
              <Stat label="Connected" value={status.connected ? "Yes" : "No"} color={status.connected ? colors.pass : colors.fail} />
              <Stat label="Sentences Parsed" value={status.sentences_parsed.toLocaleString()} color={colors.steelLight} />
              <Stat label="Rejected" value={status.sentences_rejected.toLocaleString()} color={status.sentences_rejected > 0 ? colors.warn : colors.steelLight} />
              <Stat label="Fix Quality" value={fix?.label ?? "—"} color={fix?.color ?? colors.steelLight} />
            </div>
          )}

          {status?.last_error && (
            <div className="rounded-md border p-2 text-[10px]" style={{ borderColor: `${colors.warn}40`, background: `${colors.warn}10`, color: colors.warn }}>
              <strong>Last error:</strong> {status.last_error}
            </div>
          )}

          {/* Position details */}
          {position && (
            <div className="grid grid-cols-2 gap-3">
              <div className="rounded-md border border-navy-border bg-navy-base p-3">
                <div className="mb-2 text-[10px] font-semibold uppercase tracking-wider text-steel-gray">Position</div>
                <div className="space-y-1 font-mono text-xs">
                  <Row label="Latitude" value={position.latitude != null ? position.latitude.toFixed(8) + "°" : "—"} />
                  <Row label="Longitude" value={position.longitude != null ? position.longitude.toFixed(8) + "°" : "—"} />
                  <Row label="Altitude" value={position.altitude_m != null ? position.altitude_m.toFixed(3) + " m" : "—"} />
                  <Row label="Satellites" value={position.satellites?.toString() ?? "—"} />
                  <Row label="HDOP" value={position.hdop?.toFixed(2) ?? "—"} />
                </div>
              </div>
              <div className="rounded-md border border-navy-border bg-navy-base p-3">
                <div className="mb-2 text-[10px] font-semibold uppercase tracking-wider text-steel-gray">Motion</div>
                <div className="space-y-1 font-mono text-xs">
                  <Row label="Speed" value={position.speed_mps != null ? (position.speed_mps * 3.6).toFixed(2) + " km/h" : "—"} />
                  <Row label="Course" value={position.course_deg != null ? position.course_deg.toFixed(1) + "°" : "—"} />
                  <Row label="Diff Age" value={position.age_of_diff_s != null ? position.age_of_diff_s.toFixed(1) + " s" : "—"} />
                  <Row label="Station ID" value={position.diff_station_id?.toString() ?? "—"} />
                  <Row label="Timestamp" value={position.timestamp > 0 ? new Date(position.timestamp * 1000).toISOString().slice(11, 19) + "Z" : "—"} />
                </div>
              </div>
            </div>
          )}

          {/* Trail sparkline */}
          <div className="rounded-md border border-navy-border bg-navy-base p-3">
            <div className="mb-2 text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
              Position Trail ({trailPoints.length} points)
            </div>
            <svg viewBox={`0 0 ${W} ${H}`} className="w-full" style={{ maxHeight: "200px" }}>
              {/* Grid */}
              {[0.25, 0.5, 0.75].map((t) => (
                <g key={t}>
                  <line x1={pad} y1={pad + t * (H - 2 * pad)} x2={W - pad} y2={pad + t * (H - 2 * pad)} stroke={colors.border} strokeWidth="0.5" />
                  <line x1={pad + t * (W - 2 * pad)} y1={pad} x2={pad + t * (W - 2 * pad)} y2={H - pad} stroke={colors.border} strokeWidth="0.5" />
                </g>
              ))}
              {/* Trail */}
              {trailPoints.length >= 2 && (
                <polyline
                  points={trailPoints.map(p => `${lonToX(p.longitude!).toFixed(1)},${latToY(p.latitude!).toFixed(1)}`).join(" ")}
                  fill="none"
                  stroke={colors.marine}
                  strokeWidth="1.5"
                  opacity="0.6"
                />
              )}
              {/* Current position */}
              {trailPoints.length >= 1 && (
                <circle
                  cx={lonToX(trailPoints[trailPoints.length - 1].longitude!)}
                  cy={latToY(trailPoints[trailPoints.length - 1].latitude!)}
                  r="4"
                  fill={fix?.color ?? colors.fail}
                  stroke={colors.white}
                  strokeWidth="1"
                />
              )}
            </svg>
            <div className="mt-1 text-[9px] text-steel-gray text-center">
              Lon → · Lat ↑ · Scale auto-fit to trail bounds
            </div>
          </div>
        </div>

        <div className="flex items-center justify-between border-t border-navy-border px-5 py-3">
          <div className="text-[10px] text-steel-gray">
            NMEA 0183 · GGA / RMC / GLL / GSA / VTG · 5 Hz polling
          </div>
          <button
            onClick={onClose}
            className="rounded-md px-4 py-1.5 text-xs font-medium"
            style={{ background: colors.steelGray, color: colors.navyBase }}
          >
            Close
          </button>
        </div>
      </div>
    </div>
  );
}

function Stat({ label, value, color }: { label: string; value: string; color: string }) {
  return (
    <div className="rounded-md border p-2" style={{ borderColor: `${color}40`, background: `${color}10` }}>
      <div className="text-[9px] uppercase tracking-wider" style={{ color }}>{label}</div>
      <div className="mt-0.5 font-mono text-xs font-bold text-white">{value}</div>
    </div>
  );
}

function Row({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex justify-between">
      <span className="text-steel-gray">{label}</span>
      <span className="text-white">{value}</span>
    </div>
  );
}
