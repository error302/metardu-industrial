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
import { colors } from "@/lib/tokens";

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
      {/* North arrow — top-left, below the OL Zoom control */}
      <div
        className="pointer-events-none absolute left-3 top-20 z-10 flex flex-col items-center"
        title={`Map rotation: ${rotationDeg.toFixed(1)}°`}
      >
        <div
          className="flex h-10 w-10 items-center justify-center rounded-full border bg-navy-base/85 backdrop-blur"
          style={{ borderColor: colors.border }}
        >
          <Navigation
            className="h-5 w-5 transition-transform"
            style={{
              color: colors.accent,
              transform: `rotate(${rotationDeg}deg)`,
            }}
          />
        </div>
        <span className="mt-0.5 text-[9px] font-bold text-white" style={{ color: colors.accent }}>N</span>
      </div>

      {/* Layer toggle — bottom-left, above the OL ScaleLine */}
      <div className="absolute left-3 bottom-16 z-10">
        {expanded ? (
          <div className="rounded-md border bg-navy-base/90 backdrop-blur p-2 shadow-lg" style={{ borderColor: colors.border }}>
            <div className="mb-1.5 flex items-center justify-between">
              <span className="flex items-center gap-1 text-[10px] font-semibold uppercase tracking-wider text-steel-light">
                <Layers className="h-3 w-3" /> Layers
              </span>
              <button
                onClick={() => setExpanded(false)}
                className="text-steel-gray hover:text-white text-[10px]"
                aria-label="Collapse layers"
              >
                −
              </button>
            </div>
            <div className="space-y-0.5">
              {layers.map((layer) => (
                <button
                  key={layer.id}
                  onClick={() => onToggleLayer(layer.id)}
                  className="flex w-full items-center gap-2 rounded px-2 py-1 text-[10px] text-steel-light hover:bg-navy-elevated transition-colors"
                  aria-label={`${layer.visible ? "Hide" : "Show"} ${layer.label}`}
                >
                  <LayerIcon icon={layer.icon} />
                  <span className="flex-1 text-left truncate">{layer.label}</span>
                  {layer.visible ? (
                    <Eye className="h-3 w-3" style={{ color: colors.pass }} />
                  ) : (
                    <EyeOff className="h-3 w-3 text-steel-gray" />
                  )}
                </button>
              ))}
            </div>
          </div>
        ) : (
          <button
            onClick={() => setExpanded(true)}
            className="flex items-center gap-1.5 rounded-md border bg-navy-base/85 backdrop-blur px-2.5 py-1.5 text-[10px] text-steel-light hover:bg-navy-elevated transition-colors shadow"
            style={{ borderColor: colors.border }}
            aria-label="Expand layers"
            title="Toggle layers"
          >
            <Layers className="h-3 w-3" />
            <span className="font-medium">Layers</span>
            <span className="rounded px-1 text-[9px]" style={{ background: colors.elevated, color: colors.textSecondary }}>
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
