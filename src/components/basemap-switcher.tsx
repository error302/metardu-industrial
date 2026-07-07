/**
 * Basemap switcher — Sprint 17.
 *
 * Lets the surveyor switch between basemap layers:
 *   - OpenStreetMap (default, streets)
 *   - Satellite (ESRI World Imagery, free, no API key)
 *   - Terrain (OpenTopoMap, free)
 *   - Blank (no basemap — for when the survey data is the only thing that matters)
 *
 * The selection is persisted in localStorage and applied to the OL map
 * by swapping the base TileLayer source.
 */

import { useState, useCallback } from "react";
import type Map from "ol/Map";
import TileLayer from "ol/layer/Tile";
import OSM from "ol/source/OSM";
import XYZ from "ol/source/XYZ";
import { Map as MapIcon, Satellite, Mountain, Grid3x3 } from "lucide-react";
import { colors } from "@/lib/tokens";
import { Tooltip } from "@/components/tooltip";

export type BasemapType = "osm" | "satellite" | "terrain" | "blank";

const STORAGE_KEY = "metardu-basemap";

interface BasemapOption {
  type: BasemapType;
  label: string;
  icon: typeof MapIcon;
  url?: string;
  attributions?: string;
}

const BASEMAP_OPTIONS: BasemapOption[] = [
  {
    type: "osm",
    label: "Streets",
    icon: MapIcon,
  },
  {
    type: "satellite",
    label: "Satellite",
    icon: Satellite,
    url: "https://server.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer/tile/{z}/{y}/{x}",
    attributions: "© Esri World Imagery",
  },
  {
    type: "terrain",
    label: "Terrain",
    icon: Mountain,
    url: "https://{a-c}.tile.opentopomap.org/{z}/{x}/{y}.png",
    attributions: "© OpenTopoMap (CC-BY-SA)",
  },
  {
    type: "blank",
    label: "Blank",
    icon: Grid3x3,
  },
];

function getSavedBasemap(): BasemapType {
  if (typeof window === "undefined") return "osm";
  return (localStorage.getItem(STORAGE_KEY) as BasemapType) || "osm";
}

function saveBasemap(type: BasemapType) {
  localStorage.setItem(STORAGE_KEY, type);
}

/**
 * Apply a basemap to an OL map. Removes the existing base layer(s)
 * and adds a new one. Called by the map canvas when the basemap changes.
 */
export function applyBasemap(map: Map | null, type: BasemapType) {
  if (!map) return;

  // Find and remove existing base layers (TileLayers at the bottom)
  const layers = map.getLayers().getArray();
  const baseLayers = layers.filter((l) => l instanceof TileLayer);
  for (const layer of baseLayers) {
    map.removeLayer(layer);
  }

  // Add the new base layer at index 0
  let newLayer: TileLayer | null = null;
  switch (type) {
    case "osm":
      newLayer = new TileLayer({ source: new OSM() });
      break;
    case "satellite":
      newLayer = new TileLayer({
        source: new XYZ({
          url: "https://server.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer/tile/{z}/{y}/{x}",
          attributions: "© Esri World Imagery",
          maxZoom: 19,
        }),
      });
      break;
    case "terrain":
      newLayer = new TileLayer({
        source: new XYZ({
          url: "https://{a-c}.tile.opentopomap.org/{z}/{x}/{y}.png",
          attributions: "© OpenTopoMap (CC-BY-SA)",
          maxZoom: 17,
        }),
      });
      break;
    case "blank":
      // No base layer
      break;
  }

  if (newLayer) {
    map.getLayers().insertAt(0, newLayer);
  }
}

interface Props {
  map: Map | null;
  /** Called when the basemap changes, so the map canvas can apply it. */
  onBasemapChange?: (type: BasemapType) => void;
}

export function BasemapSwitcher({ map, onBasemapChange }: Props) {
  const [current, setCurrent] = useState<BasemapType>(getSavedBasemap());

  const handleSelect = useCallback((type: BasemapType) => {
    setCurrent(type);
    saveBasemap(type);
    applyBasemap(map, type);
    onBasemapChange?.(type);
  }, [map, onBasemapChange]);

  return (
    <div className="flex items-center gap-1 rounded-md border bg-navy-base/85 p-1 backdrop-blur" style={{ borderColor: colors.border }}>
      {BASEMAP_OPTIONS.map((option) => {
        const Icon = option.icon;
        const isActive = current === option.type;
        return (
          <Tooltip key={option.type} text={option.label} position="top" delay={300}>
            <button
              onClick={() => handleSelect(option.type)}
              className="rounded p-1.5 transition-colors"
              style={{
                background: isActive ? colors.accent : "transparent",
                color: isActive ? colors.navyBase : colors.steelLight,
              }}
              aria-label={option.label}
              aria-pressed={isActive}
            >
              <Icon className="h-3.5 w-3.5" />
            </button>
          </Tooltip>
        );
      })}
    </div>
  );
}
