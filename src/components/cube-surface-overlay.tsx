/**
 * CUBE Surface Raster Overlay — renders a CUBE depth grid as a colored
 * raster layer on the OpenLayers map.
 *
 * Uses a blue depth ramp: shallow = light cyan, deep = dark navy.
 * The grid is rendered to a canvas and displayed as an OL ImageLayer.
 */

import { useEffect, useRef } from "react";
import Map from "ol/Map";
import ImageLayer from "ol/layer/Image";
import type ImageStatic from "ol/source/ImageStatic";
import { fromLonLat } from "ol/proj";
import type { CubeSurfaceRpc } from "@/lib/tauri-ipc";
import { colors } from "@/lib/tokens";

interface CubeSurfaceOverlayProps {
  map: Map | null;
  surface: CubeSurfaceRpc | null;
}

/** Blue depth ramp — shallow (light) to deep (dark). */
function depthToColor(
  depth: number,
  minDepth: number,
  maxDepth: number,
): [number, number, number, number] {
  if (Number.isNaN(depth)) return [0, 0, 0, 0]; // transparent for empty cells
  const t = (depth - minDepth) / (maxDepth - minDepth || 1);
  // Light cyan (shallow) → deep navy (deep)
  const r = Math.round(20 + (10 - 20) * t);
  const g = Math.round(180 + (30 - 180) * t);
  const b = Math.round(200 + (60 - 200) * t);
  return [r, g, b, 180]; // alpha 180 for semi-transparency
}

export function CubeSurfaceOverlay({ map, surface }: CubeSurfaceOverlayProps) {
  const layerRef = useRef<ImageLayer<ImageStatic> | null>(null);

  useEffect(() => {
    if (!map) return;

    // Remove existing layer if any
    if (layerRef.current) {
      map.removeLayer(layerRef.current);
      layerRef.current = null;
    }

    if (!surface) return;
    if (!surface.dims || !surface.bounds || !surface.depths) return;

    const cols = surface.dims[0] ?? 0;
    const rows = surface.dims[1] ?? 0;
    if (cols === 0 || rows === 0) return;
    const minX = surface.bounds[0] ?? 0;
    const minY = surface.bounds[1] ?? 0;
    const maxX = surface.bounds[2] ?? 0;
    const maxY = surface.bounds[3] ?? 0;

    // Find valid depth range (excluding NaN)
    const validDepths = surface.depths.filter((d) => !Number.isNaN(d));
    if (validDepths.length === 0) return;
    const minDepth = Math.min(...validDepths);
    const maxDepth = Math.max(...validDepths);

    // Render depth grid to a canvas
    const canvas = document.createElement("canvas");
    canvas.width = cols;
    canvas.height = rows;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const imageData = ctx.createImageData(cols, rows);
    for (let i = 0; i < surface.depths.length; i++) {
      const [r, g, b, a] = depthToColor(surface.depths[i], minDepth, maxDepth);
      // The grid is row-major: index = row * cols + col
      // Canvas is also row-major but Y is flipped (row 0 = top)
      // In geographic coords, minY is bottom. So we need to flip Y.
      const row = Math.floor(i / cols);
      const flippedRow = rows - 1 - row;
      const col = i % cols;
      const pixelIdx = (flippedRow * cols + col) * 4;
      imageData.data[pixelIdx] = r;
      imageData.data[pixelIdx + 1] = g;
      imageData.data[pixelIdx + 2] = b;
      imageData.data[pixelIdx + 3] = a;
    }
    ctx.putImageData(imageData, 0, 0);

    // Create an OL ImageStatic-like layer using a custom canvas projection
    // The surface bounds are in the source CRS (assumed WGS84 / lon/lat for Phase 2)
    // We project the bounds to the map's current CRS
    const proj = map.getView().getProjection();
    const minXY = fromLonLat([minX, minY], proj);
    const maxXY = fromLonLat([maxX, maxY], proj);
    const extent = [minXY[0], minXY[1], maxXY[0], maxXY[1]];

    // Use a static image approach — convert canvas to data URL
    const dataUrl = canvas.toDataURL("image/png");

    // Dynamically import to avoid bundling issues
    import("ol/source/ImageStatic").then(({ default: ImageStatic }) => {
      const source = new ImageStatic({
        url: dataUrl,
        imageExtent: extent,
        projection: proj,
      });

      const layer = new ImageLayer({
        source,
        opacity: 0.75,
      });

      map.addLayer(layer);
      layerRef.current = layer;

      // Fit view to the surface extent
      map.getView().fit(extent, { padding: [80, 80, 80, 80], maxZoom: 14 });
    });

    return () => {
      if (layerRef.current && map) {
        map.removeLayer(layerRef.current);
        layerRef.current = null;
      }
    };
  }, [map, surface]);

  // Render a depth legend
  if (!surface) return null;

  const validDepths = surface.depths.filter((d) => !Number.isNaN(d));
  if (validDepths.length === 0) return null;
  const minDepth = Math.min(...validDepths);
  const maxDepth = Math.max(...validDepths);

  if (!surface) return null;

  return (
    <div className="pointer-events-none absolute bottom-12 left-3 z-20 rounded-md border border-navy-border bg-navy-base/90 px-3 py-2 backdrop-blur">
      <div className="mb-1 text-[9px] font-semibold uppercase tracking-wider text-steel-light">
        CUBE Surface
      </div>
      <div className="flex items-center gap-2">
        <div
          className="h-16 w-3 rounded-sm"
          style={{
            background: `linear-gradient(to top, ${colors.marineDeep}, ${colors.marineTurquoise}, ${colors.marineCyan})`,
          }}
        />
        <div className="flex flex-col justify-between text-[9px] font-mono text-steel-light" style={{ height: "64px" }}>
          <span>{maxDepth.toFixed(1)}m</span>
          <span>{((minDepth + maxDepth) / 2).toFixed(1)}m</span>
          <span>{minDepth.toFixed(1)}m</span>
        </div>
      </div>
      <div className="mt-1 text-[8px] text-steel-gray">
        {surface.dims[0]}×{surface.dims[1]} · {surface.valid_cells} cells
      </div>
    </div>
  );
}
