/**
 * OpenLayers map canvas for MetaRDU Industrial.
 *
 * Per ARCHITECTURE.md §8 — OpenLayers 10 is the PRIMARY 2D canvas.
 * Features used here:
 *   - ol/proj/proj4 integration for custom CRS (mine grids, marine datums)
 *   - ol/control/MousePosition for monospaced coordinate readout
 *   - ol/control/ScaleLine
 *   - WMS basemap (OSM standard as default, no API key)
 *   - Graticule for survey-grade grid overlay
 *   - Per-domain accent on crosshair and grid color
 *   - Survey layer rendering dropped-file bounds as vector rectangles
 */

import { useEffect, useMemo, useRef } from "react";
import Map from "ol/Map";
import View from "ol/View";
import TileLayer from "ol/layer/Tile";
import VectorLayer from "ol/layer/Vector";
import VectorSource from "ol/source/Vector";
import ImageLayer from "ol/layer/Image";
import Static from "ol/source/ImageStatic";
import OSM from "ol/source/OSM";
import { fromLonLat, transformExtent, transform } from "ol/proj";
import { MousePosition, ScaleLine, FullScreen, Zoom } from "ol/control";
import { createStringXY } from "ol/coordinate";
// Graticule import removed — OL Graticule crashes on null projection extent
import { Style, Stroke, Fill, Text as TextStyle } from "ol/style";
import Feature from "ol/Feature";
import Polygon from "ol/geom/Polygon";
import "ol/ol.css";

import { rawColors, domainAccent, rawDomainAccent, type DomainMode } from "@/lib/tokens";
import { registerEpsg, getOlProjection } from "@/lib/crs-registry";
import { useSurveyStore } from "@/stores/survey-store";
import { renderDem } from "@/lib/tauri-ipc";

interface MapCanvasProps {
  domain: DomainMode;
  epsg: string;
  /** Called once after the OL Map instance is created. */
  onMapReady?: (map: Map) => void;
}

export function MapCanvas({ domain, epsg, onMapReady }: MapCanvasProps) {
  const mapRef = useRef<HTMLDivElement>(null);
  const mapInstanceRef = useRef<Map | null>(null);
  const surveySourceRef = useRef<VectorSource | null>(null);
  const files = useSurveyStore((s) => s.files);

  useEffect(() => {
    if (!mapRef.current) return;

    // Use RAW hex colors for OpenLayers — CSS variables don't work
    // in Canvas/SVG contexts (Stroke, Fill).
    const accent = rawDomainAccent[domain].primary;

    // Empty vector layer — future home of survey features
    const surveySource = new VectorSource();
    surveySourceRef.current = surveySource;
    const surveyLayer = new VectorLayer({
      source: surveySource,
      style: new Style({
        stroke: new Stroke({ color: accent, width: 2 }),
        fill: new Fill({ color: `${accent}15` }),
      }),
    });

    // Register the user's EPSG via proj4js (async, but we don't block map init)
    // We default to EPSG:3857 for the initial view; if registration succeeds
    // the view will be updated via the epsg-change effect below.
    registerEpsg(epsg).catch((err) => {
      console.warn(`Failed to register ${epsg}, falling back to EPSG:3857`, err);
    });

    const view = new View({
      center: fromLonLat([0, 0]),
      zoom: 2,
      projection: "EPSG:3857",
    });

    const map = new Map({
      target: mapRef.current,
      layers: [
        new TileLayer({
          source: new OSM(),
          opacity: 0.65,
        }),
        // Graticule removed — it crashes on certain projections when
        // the projection extent is null. The graticule is cosmetic
        // (grid lines on the map) and not essential for functionality.
        // Can be re-added with a safe wrapper once OL fixes the issue.
        surveyLayer,
      ],
      view,
      controls: [],
    });

    map.addControl(new Zoom());
    map.addControl(new FullScreen());
    map.addControl(new ScaleLine({ units: "metric" }));

    const mousePosition = new MousePosition({
      coordinateFormat: createStringXY(6),
      projection: epsg,
      className: "metardu-mouse-position",
    });
    map.addControl(mousePosition);

    mapInstanceRef.current = map;
    onMapReady?.(map);

    return () => {
      map.setTarget(undefined);
      mapInstanceRef.current = null;
      surveySourceRef.current = null;
    };
    // ⚠️ This effect intentionally has an EMPTY dependency array. The
    // previous code had [domain] here, which destroyed and rebuilt the
    // entire OpenLayers Map instance (OSM tile layer, graticule, vector
    // source, all controls, view center/zoom) every time the user
    // toggled mining↔marine↔both. The only thing that actually depends
    // on `domain` is the accent color, which is handled by the
    // separate effect below that updates styles in place. Tearing down
    // the map lost the user's pan/zoom state and re-fetched OSM tiles
    // from the network on every toggle — a 200-500ms flicker for no
    // benefit. eslint-disable-next-line is required because `domain`
    // and `epsg` are used inside the effect but intentionally not in
    // the dep array.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // When EPSG changes, re-register and update view + MousePosition
  useEffect(() => {
    const map = mapInstanceRef.current;
    if (!map) return;

    let cancelled = false;
    (async () => {
      try {
        await registerEpsg(epsg);
        if (cancelled) return;

        const proj = getOlProjection(epsg);
        if (!proj) {
          console.warn(`OL projection ${epsg} not available`);
          return;
        }

        // Preserve the current center, reprojected to the new CRS
        const oldView = map.getView();
        const oldCenter = oldView.getCenter();
        const oldZoom = oldView.getZoom();
        const oldProj = oldView.getProjection();
        let newCenter: [number, number] = [0, 0];
        if (oldCenter) {
          try {
            newCenter = transformExtent(
              [oldCenter[0], oldCenter[1], oldCenter[0], oldCenter[1]],
              oldProj,
              proj,
            ).slice(0, 2) as [number, number];
          } catch {
            newCenter = [0, 0];
          }
        }

        map.setView(
          new View({
            projection: proj,
            center: newCenter,
            zoom: oldZoom ?? 2,
          }),
        );

        // Refresh MousePosition projection
        const controls = map.getControls().getArray();
        const mp = controls.find((c) => c instanceof MousePosition) as
          | MousePosition
          | undefined;
        if (mp) {
          mp.setProjection(proj);
        }
      } catch (err) {
        console.warn(`EPSG registration failed for ${epsg}:`, err);
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [epsg]);

  // When survey files change, render their bounds as rectangles.
  // Only display bounds whose source projection is known.
  useEffect(() => {
    const source = surveySourceRef.current;
    if (!source) return;

    source.clear();
    if (files.length === 0) return;

    const accent = rawDomainAccent[domain].primary;
    files.forEach((f) => {
      const map = mapInstanceRef.current;
      if (!f.bounds || !f.epsg || !map || f.status !== "loaded") return;
      const sourceProjection = getOlProjection(`EPSG:${f.epsg}`);
      if (!sourceProjection) return;
      const b = f.bounds;
      if (![b.min_x, b.min_y, b.max_x, b.max_y].every(Number.isFinite) || b.min_x >= b.max_x || b.min_y >= b.max_y) return;
      const coords: [number, number][] = [
          [b.min_x, b.min_y],
          [b.max_x, b.min_y],
          [b.max_x, b.max_y],
          [b.min_x, b.max_y],
          [b.min_x, b.min_y],
        ];
      let mapCoords: number[][];
      try {
        mapCoords = coords.map((coordinate) => transform(coordinate, sourceProjection, map.getView().getProjection()));
      } catch {
        return;
      }
      if (mapCoords.some((coordinate) => !coordinate.every(Number.isFinite))) return;
      const feature = new Feature({
        geometry: new Polygon([mapCoords]),
        kind: "file-bounds",
        fileName: f.name,
        fileId: f.id,
        fileKind: f.kind,
        pointCount: f.pointCount,
        lasVersion: f.lasVersion,
        pdrf: f.pdrf,
      });
      const label = f.name;
      feature.setStyle(
        new Style({
          stroke: new Stroke({ color: accent, width: 2 }),
          fill: new Fill({ color: `${accent}25` }),
          text: new TextStyle({
            text: label,
            font: "11px JetBrains Mono, monospace",
            fill: new Fill({ color: rawColors.white }),
            stroke: new Stroke({ color: rawColors.navyBase, width: 3 }),
            offsetY: -12,
          }),
        }),
      );
      feature.setId(f.id);
      source.addFeature(feature);
    });
  }, [files, domain, epsg]);

  // Zoom to fit all features when files change
  useEffect(() => {
    const map = mapInstanceRef.current;
    const source = surveySourceRef.current;
    if (!map || !source) return;
    if (source.getFeatures().length === 0) return;

    const extent = source.getExtent();
    if (!extent || extent.some((v) => !Number.isFinite(v))) return;
    map.getView().fit(extent, { padding: [80, 80, 80, 80], maxZoom: 8 });
  }, [files]);

  // ── DEM rendering: when a GeoTIFF file is loaded, render it as
  // a hillshaded color-ramp image overlay on the map. ──
  const demLayerRef = useRef<ImageLayer<Static> | null>(null);

  // Derive a stable string identifier for the loaded GeoTIFF. The
  // previous effect depended on the whole `files` array, which changes
  // reference on every file add/remove/probe-status change. That meant
  // adding a CSV while a 25M-cell DEM was rendered would re-run the
  // entire Rust render + IPC + canvas-to-PNG + ImageLayer rebuild —
  // a 3-10 second freeze for no reason. By depending on just the
  // loaded GeoTIFF's path string, the effect only re-runs when the
  // actual GeoTIFF changes.
  const loadedGeotiff = useMemo(
    () =>
      files.find((f) => f.kind === "geotiff" && f.status === "loaded" && f.epsg) ?? null,
    [files],
  );

  useEffect(() => {
    const map = mapInstanceRef.current;
    if (!map) return;

    // Remove any existing DEM layer
    if (demLayerRef.current) {
      map.removeLayer(demLayerRef.current);
      demLayerRef.current = null;
    }

    if (!loadedGeotiff?.epsg) return;

    let cancelled = false;

    (async () => {
      try {
        const sourceCrs = `EPSG:${loadedGeotiff.epsg}`;
        await registerEpsg(sourceCrs);
        if (cancelled || !getOlProjection(sourceCrs)) return;
        const result = await renderDem({
          path: loadedGeotiff.path,
          color_ramp: "terrain",
        });

        if (cancelled || !result) return;

        // Create a data URL from the RGBA bytes
        const canvas = document.createElement("canvas");
        canvas.width = result.width;
        canvas.height = result.height;
        const ctx = canvas.getContext("2d");
        if (!ctx) return;

        const imageData = ctx.createImageData(result.width, result.height);
        const rgba = new Uint8ClampedArray(result.rgba);
        imageData.data.set(rgba);
        ctx.putImageData(imageData, 0, 0);

        const dataUrl = canvas.toDataURL("image/png");

        // The raster bounds are expressed in the raster's source CRS.
        if (!result.bounds || result.bounds.length < 4) return;
        const minX = result.bounds[0];
        const minY = result.bounds[1];
        const maxX = result.bounds[2];
        const maxY = result.bounds[3];
        const projExtent = transformExtent(
          [minX, minY, maxX, maxY],
          sourceCrs,
          map.getView().getProjection(),
        );

        // Create ImageStatic source with the rendered DEM
        const source = new Static({
          url: dataUrl,
          imageExtent: projExtent,
          interpolate: true,
        });

        const layer = new ImageLayer({
          source,
          opacity: 0.85,
        });

        if (!cancelled) {
          map.addLayer(layer);
          demLayerRef.current = layer;

          // Zoom to the DEM extent
          map.getView().fit(projExtent, { padding: [80, 80, 80, 80], maxZoom: 10 });
        }
      } catch (err) {
        console.warn("DEM render failed:", err);
      }
    })();

    return () => {
      cancelled = true;
      if (demLayerRef.current && mapInstanceRef.current) {
        mapInstanceRef.current.removeLayer(demLayerRef.current);
        demLayerRef.current = null;
      }
    };
  }, [loadedGeotiff, epsg]);

  return (
    <div className="relative h-full w-full">
      <div ref={mapRef} className="h-full w-full" />

      {/* Overlay: empty-state hint */}
      {files.length === 0 && (
        <div className="pointer-events-none absolute inset-0 flex items-center justify-center p-4">
          <div className="max-w-md rounded-xl border border-[#263345] bg-[#0B0F17]/90 p-6 text-center shadow-2xl backdrop-blur-md">
            <div className="mx-auto mb-3 flex h-11 w-11 items-center justify-center rounded-xl border border-[#263345] bg-[#111827] text-[#F97316]">
              <svg className="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M9 20l-5.447-2.724A1 1 0 013 16.382V5.618a1 1 0 011.447-.894L9 7m0 13l6-3m-6 3V7m6 10l4.553 2.276A1 1 0 0021 18.382V7.618a1 1 0 00-.553-.894L15 4m0 13V4m0 0L9 7" />
              </svg>
            </div>
            <div className="font-mono text-[10px] uppercase tracking-[0.25em] text-slate-500">
              OpenLayers 10 · {epsg}
            </div>
            <div className="mt-1 text-base font-medium text-slate-200">
              Survey Canvas Ready
            </div>
            <p className="mt-1.5 text-xs text-slate-400 leading-relaxed">
              Drag & drop a <span className="font-mono text-slate-300">.LAS</span>,{" "}
              <span className="font-mono text-slate-300">.GeoTIFF</span>, or{" "}
              <span className="font-mono text-slate-300">.all</span> file anywhere onto the canvas, or create a survey project from the left menu.
            </p>
            <div className="mt-4 flex items-center justify-center gap-2">
              <span
                className="inline-flex items-center gap-1.5 rounded-full px-2.5 py-0.5 text-[10px] font-semibold uppercase tracking-wider"
                style={{
                  background: `${domainAccent[domain].primary}15`,
                  color: domainAccent[domain].primary,
                  border: `1px solid ${domainAccent[domain].primary}30`,
                }}
              >
                <span className="h-1.5 w-1.5 rounded-full" style={{ background: domainAccent[domain].primary }} />
                {domainAccent[domain].label} Mode
              </span>
            </div>
          </div>
        </div>
      )}

      <style>{`
        .metardu-mouse-position {
          position: absolute !important;
          bottom: 12px;
          left: 105px;
          background: rgba(11, 15, 23, 0.9) !important;
          color: #9CA3AF !important;
          font-family: "JetBrains Mono", monospace !important;
          font-size: 11px !important;
          padding: 3px 8px !important;
          border-radius: 4px !important;
          border: 1px solid #263345 !important;
          font-variant-numeric: tabular-nums;
          pointer-events: none;
          z-index: 10;
        }
        .ol-control {
          background: rgba(11, 15, 23, 0.9) !important;
          border: 1px solid #263345 !important;
          border-radius: 6px !important;
          padding: 2px !important;
          box-shadow: 0 2px 6px rgba(0,0,0,0.3) !important;
        }
        .ol-control button {
          background: transparent !important;
          color: #9CA3AF !important;
          font-size: 13px !important;
          width: 26px !important;
          height: 26px !important;
          border-radius: 4px !important;
          transition: all 120ms ease !important;
        }
        .ol-control button:hover {
          background: #1F2937 !important;
          color: #F3F4F6 !important;
        }
        .ol-scale-line {
          background: rgba(11, 15, 23, 0.9) !important;
          border-color: #263345 !important;
          color: #9CA3AF !important;
          border-radius: 4px !important;
          padding: 2px 6px !important;
        }
        .ol-scale-line-inner {
          color: #9CA3AF !important;
          border-color: #4B5563 !important;
          font-family: "JetBrains Mono", monospace !important;
          font-size: 10px !important;
        }
        .ol-zoom {
          top: 10px !important;
          left: 10px !important;
          display: flex !important;
          flex-direction: column !important;
          gap: 1px !important;
        }
        .ol-full-screen {
          top: 10px !important;
          left: 44px !important;
        }
        .ol-graticule {
          pointer-events: none;
        }
      `}</style>
    </div>
  );
}
