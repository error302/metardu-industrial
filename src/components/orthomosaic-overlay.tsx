/**
 * OrthomosaicOverlay — renders an RGB orthomosaic GeoTIFF on the OpenLayers map.
 *
 * Calls `read_orthomosaic_cmd` to get the RGB pixel data + bounds from the
 * Rust backend, converts the raw bytes to a PNG via a canvas, then creates
 * an OpenLayers ImageLayer with an ImageStatic source georeferenced to
 * the orthomosaic's world bounds.
 *
 * The layer is toggleable via the map layer toggle (Sprint 12 MapOverlays).
 */

import { useEffect, useRef, useState } from "react";
import type Map from "ol/Map";
import ImageLayer from "ol/layer/Image";
import Static from "ol/source/ImageStatic";
import { invoke } from "@tauri-apps/api/core";
import { isNative } from "@/lib/tauri-ipc";

interface Orthomosaic {
  width: number;
  height: number;
  rgb_data: number[];
  bounds: [number, number, number, number];
  crs: string;
  pixel_size: [number, number];
}

interface Props {
  map: Map | null;
  /** Path to the orthomosaic GeoTIFF. When set, loads and renders the overlay. */
  orthoPath: string | null;
  /** Whether the layer is visible (controlled by the map layer toggle). */
  visible: boolean;
  /** Called when loading completes or fails. */
  onLoadStatus?: (status: "idle" | "loading" | "loaded" | "error", message?: string) => void;
}

export function OrthomosaicOverlay({ map, orthoPath, visible, onLoadStatus }: Props) {
  const layerRef = useRef<ImageLayer<Static> | null>(null);
  const [, setLoading] = useState(false);

  useEffect(() => {
    if (!map || !orthoPath) {
      // Clean up any existing layer
      if (layerRef.current && map) {
        map.removeLayer(layerRef.current);
        layerRef.current = null;
      }
      return;
    }

    if (!isNative()) {
      onLoadStatus?.("error", "Browser mode — orthomosaic requires native Tauri shell");
      return;
    }

    let cancelled = false;
    setLoading(true);
    onLoadStatus?.("loading");

    (async () => {
      try {
        const ortho = await invoke<Orthomosaic>("read_orthomosaic_cmd", { path: orthoPath });

        if (cancelled || !map) return;

        // Convert RGB byte array to PNG via canvas
        const canvas = document.createElement("canvas");
        canvas.width = ortho.width;
        canvas.height = ortho.height;
        const ctx = canvas.getContext("2d");
        if (!ctx) {
          onLoadStatus?.("error", "Canvas 2D context unavailable");
          return;
        }

        const imageData = ctx.createImageData(ortho.width, ortho.height);
        // rgb_data is [R, G, B, R, G, B, ...] — ImageData needs [R, G, B, A, ...]
        for (let i = 0; i < ortho.rgb_data.length / 3; i++) {
          imageData.data[i * 4] = ortho.rgb_data[i * 3];         // R
          imageData.data[i * 4 + 1] = ortho.rgb_data[i * 3 + 1]; // G
          imageData.data[i * 4 + 2] = ortho.rgb_data[i * 3 + 2]; // B
          imageData.data[i * 4 + 3] = 255;                       // A (opaque)
        }
        ctx.putImageData(imageData, 0, 0);

        const pngDataUrl = canvas.toDataURL("image/png");

        // Remove old layer if it exists
        if (layerRef.current) {
          map.removeLayer(layerRef.current);
        }

        // Create ImageStatic source georeferenced to the orthomosaic bounds
        // OL imageExtent is [minX, minY, maxX, maxY]
        const extent = [ortho.bounds[0], ortho.bounds[1], ortho.bounds[2], ortho.bounds[3]];

        const source = new Static({
          url: pngDataUrl,
          imageExtent: extent,
          projection: map.getView().getProjection(),
        });

        const layer = new ImageLayer({
          source,
          opacity: 0.85,
          visible,
        });

        map.addLayer(layer);
        layerRef.current = layer;

        // Fit the view to the orthomosaic bounds
        map.getView().fit(extent, { padding: [80, 80, 80, 80], maxZoom: 18 });

        setLoading(false);
        onLoadStatus?.("loaded", `${ortho.width}×${ortho.height} pixels`);
      } catch (err) {
        if (!cancelled) {
          setLoading(false);
          onLoadStatus?.("error", err instanceof Error ? err.message : String(err));
        }
      }
    })();

    return () => {
      cancelled = true;
      if (layerRef.current && map) {
        map.removeLayer(layerRef.current);
        layerRef.current = null;
      }
    };
  }, [map, orthoPath]); // eslint-disable-line react-hooks/exhaustive-deps

  // Toggle visibility without reloading
  useEffect(() => {
    if (layerRef.current) {
      layerRef.current.setVisible(visible);
    }
  }, [visible]);

  return null; // This component only manages the OL layer; renders nothing itself
}
