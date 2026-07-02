/**
 * Profile Panel — shows elevation profile along the line drawn on the map.
 *
 * Phase 0: synthesizes elevation from a Perlin-like noise function so the
 * UX is testable without a real DEM. Phase 1+ will sample actual elevation
 * from a loaded GeoTIFF DEM via the Rust core.
 */

import { useMemo } from "react";
import { Activity } from "lucide-react";
import { colors, domainAccent, type DomainMode } from "@/lib/tokens";

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

  const profile = useMemo<ProfilePoint[]>(() => {
    if (!line) return [];
    const [start, end] = line;
    // Haversine distance for approximate meters (assumes lon/lat)
    const totalMeters = haversine(start, end);
    const pts: ProfilePoint[] = [];
    for (let i = 0; i <= SAMPLES; i++) {
      const t = i / SAMPLES;
      const lon = start[0] + (end[0] - start[0]) * t;
      const lat = start[1] + (end[1] - start[1]) * t;
      // Phase 0: synthesize elevation via simple noise
      const elev = synthesizeElevation(lon, lat);
      pts.push({ distance: totalMeters * t, elevation: elev });
    }
    return pts;
  }, [line]);

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

  return (
    <div className="flex h-full flex-col p-3">
      <div className="mb-2 flex items-center justify-between">
        <span className="text-[10px] font-semibold uppercase tracking-wider text-steel-light">
          Elevation Profile
        </span>
        <span
          className="font-mono text-[10px]"
          style={{ color: accent }}
        >
          {formatDistance(maxDist)}
        </span>
      </div>

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
              stroke={colors.navyBorder}
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
              stroke={colors.navyBorder}
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
        Phase 0: synthesized elevation · Phase 1+ samples real DEM
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
