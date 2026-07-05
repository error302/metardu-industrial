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
import { fromLonLat, transformExtent } from "ol/proj";
import { MousePosition, ScaleLine, FullScreen, Zoom } from "ol/control";
import { createStringXY } from "ol/coordinate";
import Graticule from "ol/layer/Graticule";
import { Style, Stroke, Fill, Text as TextStyle } from "ol/style";
import Feature from "ol/Feature";
import Polygon from "ol/geom/Polygon";
import "ol/ol.css";

import { colors, rawColors, domainAccent, rawDomainAccent, type DomainMode } from "@/lib/tokens";
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
        new Graticule({
          strokeStyle: new Stroke({
            color: `${accent}40`,
            width: 1,
          }),
          showLabels: true,
          wrapX: false,
          lonLabelPosition: 0.5,
          latLabelPosition: 0.95,
        }),
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
  // Uses real bounds from the LAS header probe (via Rust IPC) when
  // available. Falls back to placeholder rectangles for files that
  // haven't been probed yet or don't have parseable bounds.
  useEffect(() => {
    const source = surveySourceRef.current;
    if (!source) return;

    source.clear();
    if (files.length === 0) return;

    const accent = rawDomainAccent[domain].primary;
    files.forEach((f, idx) => {
      let coords: [number, number][] | null = null;

      if (f.bounds) {
        // Real bounds from LAS header — already in WGS84 / source CRS
        // (Phase 0 assumes WGS84; future: reproject from source CRS)
        const b = f.bounds;
        coords = [
          [b.min_x, b.min_y],
          [b.max_x, b.min_y],
          [b.max_x, b.max_y],
          [b.min_x, b.max_y],
          [b.min_x, b.min_y],
        ];
      } else {
        // Placeholder: small rectangle near (0,0) so the user sees
        // SOMETHING react when a file is dropped without real bounds yet
        const lonOffset = (idx - (files.length - 1) / 2) * 2;
        const latOffset = 0;
        const size = 1.5;
        coords = [
          [lonOffset - size / 2, latOffset - size / 2],
          [lonOffset + size / 2, latOffset - size / 2],
          [lonOffset + size / 2, latOffset + size / 2],
          [lonOffset - size / 2, latOffset + size / 2],
          [lonOffset - size / 2, latOffset - size / 2],
        ];
      }

      const mercatorCoords = coords.map(([lon, lat]) =>
        fromLonLat([lon, lat]),
      );
      const feature = new Feature({
        geometry: new Polygon([mercatorCoords]),
        kind: "file-bounds",
        fileName: f.name,
        fileId: f.id,
        fileKind: f.kind,
        pointCount: f.pointCount,
        lasVersion: f.lasVersion,
        pdrf: f.pdrf,
      });
      const label = f.pointCount
        ? `${f.name} · ${f.pointCount.toLocaleString()} pts`
        : `${f.name} · probing…`;
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
  }, [files, domain]);

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
  const loadedGeotiffPath = useMemo(
    () =>
      files.find((f) => f.kind === "geotiff" && f.status === "loaded")
        ?.path ?? null,
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

    if (!loadedGeotiffPath) return;

    let cancelled = false;

    (async () => {
      try {
        const result = await renderDem({
          path: loadedGeotiffPath,
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

        // Convert bounds from WGS84 to the map's projection
        if (!result.bounds || result.bounds.length < 4) return;
        const minX = result.bounds[0];
        const minY = result.bounds[1];
        const maxX = result.bounds[2];
        const maxY = result.bounds[3];
        const projExtent = transformExtent(
          [minX, minY, maxX, maxY],
          "EPSG:4326",
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
  }, [loadedGeotiffPath]);

  return (
    <div className="relative h-full w-full">
      <div ref={mapRef} className="h-full w-full" />

      {/* Overlay: empty-state hint */}
      {files.length === 0 && (
        <div className="pointer-events-none absolute inset-0 flex items-center justify-center">
          <div className="rounded-lg border border-navy-border bg-navy-base/80 px-6 py-4 text-center backdrop-blur-sm">
            <div className="font-mono text-[10px] tracking-[0.2em] text-steel-gray">
              OPENLAYERS 10 · {epsg}
            </div>
            <div className="mt-1 text-sm text-steel-light">
              No survey loaded. Drag a LAS / GeoTIFF / .all file here, or open
              from the menu.
            </div>
            <div
              className="mt-2 text-xs font-medium"
              style={{ color: domainAccent[domain].primary }}
            >
              {domainAccent[domain].label} mode active
            </div>
          </div>
        </div>
      )}

      <style>{`
        .metardu-mouse-position {
          position: absolute !important;
          bottom: 8px;
          left: 8px;
          background: rgba(10, 25, 47, 0.9);
          color: ${colors.industrialOrange};
          font-family: ${"JetBrains Mono"}, monospace;
          font-size: 11px;
          padding: 4px 8px;
          border-radius: 4px;
          border: 1px solid ${colors.navyBorder};
          font-variant-numeric: tabular-nums;
          pointer-events: none;
        }
        .ol-control {
          background: rgba(10, 25, 47, 0.85) !important;
          border: 1px solid ${colors.navyBorder} !important;
          border-radius: 4px !important;
          padding: 0 !important;
        }
        .ol-control button {
          background: transparent !important;
          color: ${colors.white} !important;
          font-size: 14px !important;
          width: 28px !important;
          height: 28px !important;
        }
        .ol-control button:hover {
          background: ${colors.industrialOrange} !important;
          color: ${colors.navyBase} !important;
        }
        .ol-scale-line {
          background: rgba(10, 25, 47, 0.85) !important;
          border-color: ${colors.navyBorder} !important;
          color: ${colors.steelLight} !important;
        }
        .ol-scale-line-inner {
          color: ${colors.steelLight} !important;
          border-color: ${colors.steelLight} !important;
          font-family: ${"JetBrains Mono"}, monospace !important;
        }
        .ol-zoom {
          top: 12px !important;
          left: 12px !important;
        }
        .ol-full-screen {
          top: 12px !important;
          left: 56px !important;
        }
        .ol-graticule {
          pointer-events: none;
        }
      `}</style>
    </div>
  );
}
