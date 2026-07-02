/**
 * Point cloud rendering layer using Deck.gl.
 *
 * Renders LAS point cloud data on top of the OpenLayers map. Points are
 * colored by classification:
 *   - Ground (after CSF): green (#10B981)
 *   - Non-ground: orange (#FFB347)
 *   - Unclassified (no CSF run): steel blue (#6B7280)
 *
 * The layer is embedded as an OpenLayers overlay via the deck.gl-OL
 * integration pattern: we create a Deck.gl canvas positioned absolutely
 * over the OL map, synchronized via view change events.
 */

import { useEffect, useRef, useState, useMemo } from "react";
import Map from "ol/Map";
import { Deck } from "@deck.gl/core";
import { ScatterplotLayer } from "@deck.gl/layers";
import { fromLonLat, toLonLat } from "ol/proj";
import { colors } from "@/lib/tokens";
import { readLasPoints, type CsfResult } from "@/lib/tauri-ipc";
import { useSurveyStore } from "@/stores/survey-store";

/**
 * Client-side LOD decimation — mirrors the Rust decimate_points function.
 * Uses spatial hashing: divides bounding box into cells of cell_size,
 * keeps one representative point per cell (median by Z).
 */
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

/** Determine LOD cell size based on map zoom and point count. */
function lodCellSize(zoom: number, pointCount: number): number {
  if (pointCount < 10000) return 0; // no decimation for small clouds
  if (zoom >= 16) return 0;          // close enough to show all
  if (zoom >= 14) return 1.0;        // 1m cells
  if (zoom >= 12) return 5.0;        // 5m cells
  if (zoom >= 10) return 25.0;       // 25m cells
  return 100.0;                       // world view
}

interface PointCloudLayerProps {
  map: Map | null;
  /** Active file ID to render. When null, no points are shown. */
  activeFileId: string | null;
  /** CSF classification result (if run) — drives ground/non-ground coloring. */
  csfResult: CsfResult | null;
  /** Max points to render (performance cap). Default 100000. */
  maxPoints?: number;
}

interface PointData {
  position: [number, number]; // Web Mercator [x, y]
  z: number; // elevation in meters
  isGround: boolean | null; // null = unclassified
  index: number;
}

const DEFAULT_MAX_POINTS = 100_000;

export function PointCloudLayer({
  map,
  activeFileId,
  csfResult,
  maxPoints = DEFAULT_MAX_POINTS,
}: PointCloudLayerProps) {
  const deckRef = useRef<Deck | null>(null);
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const [rawPoints, setRawPoints] = useState<[number, number, number][]>([]);
  const [loading, setLoading] = useState(false);
  const [pointCount, setPointCount] = useState(0);
  const [currentZoom, setCurrentZoom] = useState(2);
  const files = useSurveyStore((s) => s.files);

  // Load points when active file changes
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
        if (!pts) {
          setRawPoints([]);
          setPointCount(0);
          setLoading(false);
          return;
        }
        setRawPoints(pts);
        setPointCount(pts.length);
        setLoading(false);
      })
      .catch(() => {
        setRawPoints([]);
        setPointCount(0);
        setLoading(false);
      });
  }, [activeFileId, files, maxPoints, csfResult]);

  // Track zoom changes for LOD
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

  // Apply LOD decimation based on current zoom
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

  // Initialize Deck.gl canvas over the OL map
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

    // Sync Deck.gl view with OL view
    const syncView = () => {
      const view = map.getView();
      const center = view.getCenter();
      const zoom = view.getZoom();
      if (!center || zoom === undefined) return;
      const [lon, lat] = toLonLat(center);
      // OL zoom → Deck.gl zoom (approximate: OL Web Mercator zoom ≈ Deck.gl zoom)
      deck.setProps({
        initialViewState: { longitude: lon, latitude: lat, zoom: zoom, bearing: 0, pitch: 0 },
      });
    };

    map.on("moveend", syncView);
    syncView();

    return () => {
      map.un("moveend", syncView);
      deck.finalize();
      deckRef.current = null;
    };
  }, [map]);

  // Update Deck.gl layers when points change
  useEffect(() => {
    if (!deckRef.current) return;

    const layer = new ScatterplotLayer({
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
        if (p.isGround === true) {
          return [16, 185, 129, 200]; // green for ground
        } else if (p.isGround === false) {
          return [255, 179, 71, 200]; // orange for non-ground
        }
        return [107, 114, 128, 180]; // steel gray for unclassified
      },
      // Anti-aliasing for smoother point edges
      antialiasing: true,
    });

    deckRef.current.setProps({ layers: [layer] });
  }, [points]);

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
        </div>
      )}
    </>
  );
}
