/**
 * Point cloud rendering layer using Deck.gl.
 *
 * Renders LAS point cloud data on top of the OpenLayers map. Points are
 * colored by classification:
 *   - Ground (after CSF): green (#10B981)
 *   - Non-ground: orange (#FFB347)
 *   - Unclassified (no CSF run): steel blue (#6B7280)
 *
 * Also renders live-streamed pings (from UDP streaming ingest) as a
 * separate Deck.gl layer in turquoise — these update in real-time
 * without reloading the main point cloud.
 */

import { useEffect, useRef, useState, useMemo } from "react";
import Map from "ol/Map";
import { Deck } from "@deck.gl/core";
import { ScatterplotLayer } from "@deck.gl/layers";
import { fromLonLat, toLonLat } from "ol/proj";
import { colors } from "@/lib/tokens";
import { readLasPoints, type CsfResult } from "@/lib/tauri-ipc";
import { useSurveyStore } from "@/stores/survey-store";

/** Client-side LOD decimation — mirrors the Rust decimate_points function. */
function decimateClientSide(
  points: [number, number, number][],
  cellSize: number,
): [number, number, number][] {
  if (cellSize <= 0 || points.length === 0) return points;
  const cells: Record<string, [number, number, number][]> = {};
  for (const p of points) {
    const col = Math.floor(p[0] / cellSize);
    const row = Math.floor(p[1] / cellSize);
    const key = `${col},${row}`;
    if (cells[key]) cells[key].push(p);
    else cells[key] = [p];
  }
  const result: [number, number, number][] = [];
  for (const key of Object.keys(cells)) {
    const cell = cells[key];
    cell.sort((a, b) => a[2] - b[2]);
    result.push(cell[Math.floor(cell.length / 2)]);
  }
  return result;
}

function lodCellSize(zoom: number, pointCount: number): number {
  if (pointCount < 10000) return 0;
  if (zoom >= 16) return 0;
  if (zoom >= 14) return 1.0;
  if (zoom >= 12) return 5.0;
  if (zoom >= 10) return 25.0;
  return 100.0;
}

export interface StreamPing {
  x: number;
  y: number;
  depth: number;
  uncertainty: number;
  timestamp: number;
}

interface PointCloudLayerProps {
  map: Map | null;
  activeFileId: string | null;
  csfResult: CsfResult | null;
  maxPoints?: number;
  /** Live-streamed pings from UDP ingest — rendered as a separate layer */
  streamPings?: StreamPing[];
}

interface PointData {
  position: [number, number];
  z: number;
  isGround: boolean | null;
  index: number;
}

const DEFAULT_MAX_POINTS = 100_000;

export function PointCloudLayer({
  map,
  activeFileId,
  csfResult,
  maxPoints = DEFAULT_MAX_POINTS,
  streamPings = [],
}: PointCloudLayerProps) {
  const deckRef = useRef<Deck | null>(null);
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const [rawPoints, setRawPoints] = useState<[number, number, number][]>([]);
  const [loading, setLoading] = useState(false);
  const [pointCount, setPointCount] = useState(0);
  const [currentZoom, setCurrentZoom] = useState(2);
  const files = useSurveyStore((s) => s.files);

  useEffect(() => {
    if (!activeFileId) {
      setRawPoints([]);
      setPointCount(0);
      return;
    }
    const file = files.find((f) => f.id === activeFileId);
    if (!file || file.kind !== "las") {
      setRawPoints([]);
      setPointCount(0);
      return;
    }
    setLoading(true);
    readLasPoints(file.path, maxPoints)
      .then((pts) => {
        if (!pts) { setRawPoints([]); setPointCount(0); setLoading(false); return; }
        setRawPoints(pts);
        setPointCount(pts.length);
        setLoading(false);
      })
      .catch(() => { setRawPoints([]); setPointCount(0); setLoading(false); });
  }, [activeFileId, files, maxPoints, csfResult]);

  useEffect(() => {
    if (!map) return;
    const updateZoom = () => {
      const z = map.getView().getZoom();
      if (z !== undefined) setCurrentZoom(z);
    };
    map.on("moveend", updateZoom);
    updateZoom();
    return () => { map.un("moveend", updateZoom); };
  }, [map]);

  const points = useMemo<PointData[]>(() => {
    if (rawPoints.length === 0) return [];
    const cellSize = lodCellSize(currentZoom, rawPoints.length);
    const decimated = decimateClientSide(rawPoints, cellSize);
    return decimated.map((p, i) => ({
      position: fromLonLat([p[0], p[1]]) as [number, number],
      z: p[2],
      isGround: csfResult?.is_ground[i] ?? null,
      index: i,
    }));
  }, [rawPoints, currentZoom, csfResult]);

  // Convert streamed pings to Deck.gl positions
  const streamPoints = useMemo(() => {
    if (streamPings.length === 0) return [];
    return streamPings.map((p) => ({
      position: fromLonLat([p.x, p.y]) as [number, number],
      depth: p.depth,
    }));
  }, [streamPings]);

  useEffect(() => {
    if (!map || !canvasRef.current) return;
    const deck = new Deck({
      canvas: canvasRef.current,
      width: "100%",
      height: "100%",
      initialViewState: { longitude: 0, latitude: 0, zoom: 2 },
      controller: false,
      layers: [],
    });
    deckRef.current = deck;
    const syncView = () => {
      const view = map.getView();
      const center = view.getCenter();
      const zoom = view.getZoom();
      if (!center || zoom === undefined) return;
      const [lon, lat] = toLonLat(center);
      deck.setProps({
        initialViewState: { longitude: lon, latitude: lat, zoom, bearing: 0, pitch: 0 },
      });
    };
    map.on("moveend", syncView);
    syncView();
    return () => { map.un("moveend", syncView); deck.finalize(); deckRef.current = null; };
  }, [map]);

  // Update Deck.gl layers when points or stream pings change
  useEffect(() => {
    if (!deckRef.current) return;

    const layers: ScatterplotLayer[] = [];

    // Main point cloud layer (from LAS)
    if (points.length > 0) {
      layers.push(
        new ScatterplotLayer({
          id: "point-cloud",
          data: points,
          pickable: false,
          opacity: 0.8,
          stroked: false,
          filled: true,
          radiusScale: 1,
          radiusMinPixels: 1,
          radiusMaxPixels: 4,
          getPosition: (d: unknown) => {
            const p = d as PointData;
            return [p.position[0], p.position[1], 0];
          },
          getRadius: 2,
          getFillColor: (d: unknown) => {
            const p = d as PointData;
            if (p.isGround === true) return [16, 185, 129, 200];
            if (p.isGround === false) return [255, 179, 71, 200];
            return [107, 114, 128, 180];
          },
          antialiasing: true,
        }),
      );
    }

    // Live stream layer (from UDP ingest) — turquoise, larger radius
    if (streamPoints.length > 0) {
      layers.push(
        new ScatterplotLayer({
          id: "stream-pings",
          data: streamPoints,
          pickable: false,
          opacity: 0.9,
          stroked: false,
          filled: true,
          radiusScale: 1,
          radiusMinPixels: 2,
          radiusMaxPixels: 6,
          getPosition: (d: unknown) => {
            const p = d as { position: [number, number] };
            return [p.position[0], p.position[1], 0];
          },
          getRadius: 3,
          getFillColor: () => [32, 178, 170, 220], // marine turquoise
          antialiasing: true,
        }),
      );
    }

    deckRef.current.setProps({ layers });
  }, [points, streamPoints]);

  if (!map) return null;

  return (
    <>
      <canvas
        ref={canvasRef}
        className="pointer-events-none absolute inset-0 z-10"
        style={{ width: "100%", height: "100%" }}
      />
      {loading && (
        <div className="absolute bottom-4 left-1/2 z-20 -translate-x-1/2 rounded-md border border-navy-border bg-navy-base/90 px-4 py-2 text-xs backdrop-blur">
          <span className="font-mono text-steel-light">
            Loading {pointCount.toLocaleString()} points…
          </span>
        </div>
      )}
      {!loading && pointCount > 0 && (
        <div className="pointer-events-none absolute bottom-4 left-1/2 z-20 -translate-x-1/2 flex items-center gap-3 rounded-md border border-navy-border bg-navy-base/90 px-4 py-2 text-[10px] backdrop-blur">
          <span className="font-mono text-steel-light">
            {pointCount.toLocaleString()} pts
          </span>
          <span className="flex items-center gap-1">
            <span className="h-2 w-2 rounded-full" style={{ background: colors.pass }} />
            <span className="text-steel-gray">
              Ground: {csfResult ? csfResult.ground_count.toLocaleString() : "—"}
            </span>
          </span>
          <span className="flex items-center gap-1">
            <span className="h-2 w-2 rounded-full" style={{ background: colors.miningBurnt }} />
            <span className="text-steel-gray">
              Non-ground: {csfResult ? csfResult.non_ground_count.toLocaleString() : "—"}
            </span>
          </span>
          {streamPings.length > 0 && (
            <span className="flex items-center gap-1">
              <span className="h-2 w-2 rounded-full animate-pulse" style={{ background: colors.marineTurquoise }} />
              <span className="text-steel-gray">Stream: {streamPings.length}</span>
            </span>
          )}
        </div>
      )}
    </>
  );
}
