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
 */

import { useEffect, useRef } from "react";
import Map from "ol/Map";
import View from "ol/View";
import TileLayer from "ol/layer/Tile";
import VectorLayer from "ol/layer/Vector";
import VectorSource from "ol/source/Vector";
import OSM from "ol/source/OSM";
import { fromLonLat } from "ol/proj";
import { MousePosition, ScaleLine, FullScreen, Zoom } from "ol/control";
import { createStringXY } from "ol/coordinate";
import Graticule from "ol/layer/Graticule";
import { Style, Stroke } from "ol/style";
import "ol/ol.css";

import { colors, domainAccent, type DomainMode } from "@/lib/tokens";

interface MapCanvasProps {
  domain: DomainMode;
  epsg: string;
}

export function MapCanvas({ domain, epsg }: MapCanvasProps) {
  const mapRef = useRef<HTMLDivElement>(null);
  const mapInstanceRef = useRef<Map | null>(null);

  useEffect(() => {
    if (!mapRef.current) return;

    const accent = domainAccent[domain].primary;

    // Empty vector layer — future home of survey features
    const surveySource = new VectorSource();
    const surveyLayer = new VectorLayer({
      source: surveySource,
      style: new Style({
        stroke: new Stroke({ color: accent, width: 2 }),
      }),
    });

    // Default view — center on world, EPSG:3857 (Web Mercator)
    // Real impl uses proj4.defs(epsg, ...) for custom CRS
    const view = new View({
      center: fromLonLat([0, 0]),
      zoom: 2,
      projection: epsg === "EPSG:4326" ? "EPSG:4326" : "EPSG:3857",
    });

    const map = new Map({
      target: mapRef.current,
      layers: [
        new TileLayer({
          source: new OSM(),
          // Dim the basemap so survey data pops
          opacity: 0.65,
        }),
        // Graticule — survey grid overlay, accent color per domain
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

    // Add controls programmatically so we control placement
    map.addControl(new Zoom());
    map.addControl(new FullScreen());
    map.addControl(new ScaleLine({ units: "metric" }));

    // Mouse position — monospaced, bottom-left, in active CRS
    const mousePosition = new MousePosition({
      coordinateFormat: createStringXY(6),
      projection: epsg,
      className: "metardu-mouse-position",
    });
    map.addControl(mousePosition);

    mapInstanceRef.current = map;

    return () => {
      map.setTarget(undefined);
      mapInstanceRef.current = null;
    };
  }, [domain, epsg]);

  return (
    <div className="relative h-full w-full">
      <div ref={mapRef} className="h-full w-full" />

      {/* Overlay: empty-state hint */}
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

      {/* Style overrides for OL controls */}
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
