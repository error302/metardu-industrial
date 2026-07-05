/**
 * Profile Panel — shows elevation profile along the line drawn on the map.
 *
 * Phase 1: when a GeoTIFF DEM is loaded in the survey store, calls
 * sample_profile IPC to read real elevation values via the Rust core
 * (bilinear interpolation across DEM strips). Falls back to synthesized
 * elevation when no DEM is available or we're in browser mode.
 */

import { useEffect, useMemo, useState } from "react";
import { Activity, Database, Sparkles } from "lucide-react";
import { colors, rawColors, domainAccent, type DomainMode } from "@/lib/tokens";
import { sampleProfile, type ProfileSampleResult } from "@/lib/tauri-ipc";
import { useSurveyStore } from "@/stores/survey-store";

interface Props {
  domain: DomainMode;
  /** Two [lon, lat] endpoints in the active CRS */
  line: [[number, number], [number, number]] | null;
}

interface ProfilePoint {
  distance: number; // meters from start
  elevation: number; // meters
}

const SAMPLES = 200;

export function ProfilePanel({ domain, line }: Props) {
  const accent = domainAccent[domain].primary;
  const files = useSurveyStore((s) => s.files);
  const [realProfile, setRealProfile] = useState<ProfileSampleResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [realError, setRealError] = useState<string | null>(null);

  // Find the most recently loaded GeoTIFF DEM
  const demFile = useMemo(() => {
    return files.find((f) => f.kind === "geotiff" && f.status === "loaded");
  }, [files]);

  // Fetch real profile when line + DEM change
  useEffect(() => {
    if (!line || !demFile) {
      setRealProfile(null);
      return;
    }
    setLoading(true);
    setRealError(null);
    sampleProfile(
      demFile.path,
      line[0][0],
      line[0][1],
      line[1][0],
      line[1][1],
      SAMPLES,
    )
      .then((result) => {
        setRealProfile(result);
        setLoading(false);
      })
      .catch((err: unknown) => {
        setRealError(err instanceof Error ? err.message : String(err));
        setRealProfile(null);
        setLoading(false);
      });
  }, [line, demFile]);

  const profile = useMemo<ProfilePoint[]>(() => {
    if (!line) return [];
    if (realProfile && realProfile.from_real_dem) {
      // Real DEM data — distances already provided
      return realProfile.elevations.map((e, i) => ({
        distance: realProfile.distances[i] ?? 0,
        elevation: e,
      }));
    }
    // Fallback: synthesize
    const [start, end] = line;
    const totalMeters = haversine(start, end);
    const pts: ProfilePoint[] = [];
    for (let i = 0; i <= SAMPLES; i++) {
      const t = i / SAMPLES;
      const lon = start[0] + (end[0] - start[0]) * t;
      const lat = start[1] + (end[1] - start[1]) * t;
      const elev = synthesizeElevation(lon, lat);
      pts.push({ distance: totalMeters * t, elevation: elev });
    }
    return pts;
  }, [line, realProfile]);

  if (!line) {
    return (
      <div className="flex h-full items-center justify-center p-4 text-center">
        <div>
          <Activity
            className="mx-auto mb-2 h-8 w-8"
            style={{ color: colors.steelGray }}
          />
          <p className="text-xs text-steel-gray">
            Draw a line on the map to see the elevation profile.
          </p>
          <p className="mt-1 text-[10px] text-steel-gray/70">
            Click two points on the canvas.
          </p>
        </div>
      </div>
    );
  }

  // Compute viewBox bounds
  const maxDist = profile[profile.length - 1]?.distance ?? 1;
  const elevs = profile.map((p) => p.elevation);
  const minElev = Math.min(...elevs);
  const maxElev = Math.max(...elevs);
  const elevRange = maxElev - minElev || 1;
  const w = 100;
  const h = 40;
  const pathD = profile
    .map((p, i) => {
      const x = (i / (profile.length - 1)) * w;
      const y = h - ((p.elevation - minElev) / elevRange) * h;
      return `${i === 0 ? "M" : "L"}${x.toFixed(2)},${y.toFixed(2)}`;
    })
    .join(" ");

  const fromReal = realProfile?.from_real_dem ?? false;

  return (
    <div className="flex h-full flex-col p-3">
      <div className="mb-2 flex items-center justify-between">
        <span className="text-[10px] font-semibold uppercase tracking-wider text-steel-light">
          Elevation Profile
        </span>
        <span className="font-mono text-[10px]" style={{ color: accent }}>
          {formatDistance(maxDist)}
        </span>
      </div>

      {/* Source badge */}
      <div className="mb-2 flex items-center gap-1.5">
        {loading ? (
          <span className="flex items-center gap-1 rounded-sm bg-navy-elevated px-1.5 py-0.5 text-[9px] text-steel-light">
            <Activity className="h-2.5 w-2.5 animate-pulse" style={{ color: accent }} />
            Sampling DEM…
          </span>
        ) : fromReal ? (
          <span
            className="flex items-center gap-1 rounded-sm px-1.5 py-0.5 text-[9px]"
            style={{ background: `${colors.pass}20`, color: colors.pass }}
          >
            <Database className="h-2.5 w-2.5" />
            Real DEM
          </span>
        ) : (
          <span className="flex items-center gap-1 rounded-sm bg-navy-elevated px-1.5 py-0.5 text-[9px] text-steel-gray">
            <Sparkles className="h-2.5 w-2.5" />
            Synthesized
          </span>
        )}
        {demFile && (
          <span className="truncate text-[9px] text-steel-gray" title={demFile.name}>
            {demFile.name}
          </span>
        )}
      </div>

      {realError && (
        <div
          className="mb-2 rounded border px-2 py-1 text-[9px]"
          style={{ borderColor: `${colors.fail}40`, color: colors.fail }}
        >
          {realError}
        </div>
      )}

      <div className="relative flex-1">
        <svg
          viewBox={`0 0 ${w} ${h}`}
          preserveAspectRatio="none"
          className="absolute inset-0 h-full w-full"
        >
          {/* Grid lines */}
          {[0.25, 0.5, 0.75].map((t) => (
            <line
              key={`h-${t}`}
              x1="0"
              y1={h * t}
              x2={w}
              y2={h * t}
              stroke={rawColors.navyBorder}
              strokeWidth="0.2"
            />
          ))}
          {[0.25, 0.5, 0.75].map((t) => (
            <line
              key={`v-${t}`}
              x1={w * t}
              y1="0"
              x2={w * t}
              y2={h}
              stroke={rawColors.navyBorder}
              strokeWidth="0.2"
            />
          ))}

          {/* Profile area + line */}
          <path
            d={`${pathD} L${w},${h} L0,${h} Z`}
            fill={`${accent}20`}
          />
          <path
            d={pathD}
            fill="none"
            stroke={accent}
            strokeWidth="0.6"
            vectorEffect="non-scaling-stroke"
          />
        </svg>
      </div>

      <div className="mt-2 flex items-center justify-between text-[10px] text-steel-gray">
        <span>
          Min <span className="font-mono text-steel-light">{minElev.toFixed(1)}m</span>
        </span>
        <span>
          Max <span className="font-mono text-steel-light">{maxElev.toFixed(1)}m</span>
        </span>
        <span>
          Δ <span className="font-mono text-steel-light">{(maxElev - minElev).toFixed(1)}m</span>
        </span>
      </div>

      <div className="mt-1 text-center text-[9px] text-steel-gray/70">
        {fromReal
          ? "Bilinear-sampled from real DEM"
          : "Drop a GeoTIFF DEM to sample real elevations"}
      </div>
    </div>
  );
}

/** Haversine distance in meters (assumes lon/lat). */
function haversine(a: [number, number], b: [number, number]): number {
  const R = 6371000;
  const φ1 = (a[1] * Math.PI) / 180;
  const φ2 = (b[1] * Math.PI) / 180;
  const Δφ = ((b[1] - a[1]) * Math.PI) / 180;
  const Δλ = ((b[0] - a[0]) * Math.PI) / 180;
  const h =
    Math.sin(Δφ / 2) ** 2 +
    Math.cos(φ1) * Math.cos(φ2) * Math.sin(Δλ / 2) ** 2;
  return 2 * R * Math.asin(Math.sqrt(h));
}

/** Deterministic noise-based elevation for Phase 0 demos. */
function synthesizeElevation(lon: number, lat: number): number {
  // Multi-octave sine noise — looks terrain-like
  const base = 100;
  const o1 = Math.sin(lon * 10) * Math.cos(lat * 8) * 40;
  const o2 = Math.sin(lon * 25 + 0.3) * Math.cos(lat * 20 + 0.5) * 15;
  const o3 = Math.sin(lon * 80) * Math.cos(lat * 65) * 5;
  return base + o1 + o2 + o3;
}

function formatDistance(m: number): string {
  if (m < 1000) return `${m.toFixed(0)}m`;
  return `${(m / 1000).toFixed(2)}km`;
}
