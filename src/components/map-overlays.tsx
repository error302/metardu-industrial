/**
 * Map Overlays — north arrow + layer toggle (Sprint 12 UI polish).
 *
 * Standard surveying UI elements that were missing:
 *   - North arrow: required on every survey plan, shows map orientation
 *   - Layer toggle: quick show/hide of point cloud, DEM, CUBE surface
 *
 * These are React overlays positioned absolutely over the OL map —
 * not OL controls — to avoid the projection-extent crash that the
 * OL Graticule caused.
 */

import { useState } from "react";
import { Navigation, Eye, EyeOff, Layers, Mountain, Waves } from "lucide-react";

interface MapOverlayProps {
  /** Map rotation in radians (from OL view.getRotation()). 0 = North up. */
  rotation: number;
  /** Layers that can be toggled. */
  layers: MapLayer[];
  onToggleLayer: (id: string) => void;
}

export interface MapLayer {
  id: string;
  label: string;
  visible: boolean;
  icon: "pointcloud" | "dem" | "cube" | "stream";
}

export function MapOverlays({ rotation, layers, onToggleLayer }: MapOverlayProps) {
  const [expanded, setExpanded] = useState(false);

  // Convert rotation radians → degrees for the north arrow
  const rotationDeg = -rotation * (180 / Math.PI);

  return (
    <>
      {/* Sleek precision compass indicator — top-left beneath zoom */}
      <div
        className="pointer-events-none absolute left-3 top-[72px] z-10 flex items-center gap-1.5 rounded-md border border-[#263345] bg-[#0B0F17]/85 px-2 py-1 shadow-sm backdrop-blur"
        title={`Map orientation: ${rotationDeg.toFixed(1)}°`}
      >
        <Navigation
          className="h-3.5 w-3.5 transition-transform text-[#F97316]"
          style={{ transform: `rotate(${rotationDeg}deg)` }}
        />
        <span className="font-mono text-[10px] font-semibold text-slate-300">
          {rotationDeg === 0 ? "N" : `${Math.round(rotationDeg)}°`}
        </span>
      </div>

      {/* Layer toggle — bottom-left */}
      <div className="absolute left-3 bottom-14 z-10">
        {expanded ? (
          <div className="rounded-md border border-[#263345] bg-[#0B0F17]/95 p-2 shadow-xl backdrop-blur min-w-[170px]">
            <div className="mb-2 flex items-center justify-between border-b border-[#263345] pb-1">
              <span className="flex items-center gap-1.5 text-[10px] font-semibold uppercase tracking-wider text-slate-300">
                <Layers className="h-3 w-3 text-[#F97316]" /> Map Layers
              </span>
              <button
                onClick={() => setExpanded(false)}
                className="text-slate-400 hover:text-white text-xs px-1"
                aria-label="Collapse layers"
              >
                ✕
              </button>
            </div>
            <div className="space-y-1">
              {layers.map((layer) => (
                <button
                  key={layer.id}
                  onClick={() => onToggleLayer(layer.id)}
                  className="flex w-full items-center gap-2 rounded px-2 py-1 text-[11px] text-slate-300 hover:bg-[#1F2937] transition-colors"
                  aria-label={`${layer.visible ? "Hide" : "Show"} ${layer.label}`}
                >
                  <LayerIcon icon={layer.icon} />
                  <span className="flex-1 text-left truncate">{layer.label}</span>
                  {layer.visible ? (
                    <Eye className="h-3 w-3 text-emerald-400" />
                  ) : (
                    <EyeOff className="h-3 w-3 text-slate-500" />
                  )}
                </button>
              ))}
            </div>
          </div>
        ) : (
          <button
            onClick={() => setExpanded(true)}
            className="flex items-center gap-1.5 rounded-md border border-[#263345] bg-[#0B0F17]/90 px-2.5 py-1 text-[11px] text-slate-300 hover:bg-[#1F2937] transition-colors shadow-md backdrop-blur"
            aria-label="Expand layers"
            title="Toggle layers"
          >
            <Layers className="h-3 w-3 text-[#F97316]" />
            <span className="font-medium">Layers</span>
            <span className="rounded bg-[#1F2937] px-1 text-[10px] text-slate-400 font-mono">
              {layers.filter((l) => l.visible).length}/{layers.length}
            </span>
          </button>
        )}
      </div>
    </>
  );
}

function LayerIcon({ icon }: { icon: MapLayer["icon"] }) {
  switch (icon) {
    case "pointcloud":
      return <Mountain className="h-3 w-3 text-steel-gray" />;
    case "dem":
      return <Mountain className="h-3 w-3 text-steel-gray" />;
    case "cube":
      return <Waves className="h-3 w-3 text-steel-gray" />;
    case "stream":
      return <Waves className="h-3 w-3 text-steel-gray" />;
  }
}
