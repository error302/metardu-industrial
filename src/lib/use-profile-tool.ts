/**
 * Profile tool — adds click-to-draw-line interaction to the OpenLayers
 * map. Returns the drawn line (start/end coords) plus a clear fn.
 *
 * Usage: the workspace shell toggles profile mode via a button. While
 * active, clicks on the map add points (max 2 = a line). The line
 * renders on its own vector layer in the domain accent color.
 */

import { useEffect, useRef, useState } from "react";
import Map from "ol/Map";
import VectorLayer from "ol/layer/Vector";
import VectorSource from "ol/source/Vector";
import { Style, Stroke } from "ol/style";
import Feature from "ol/Feature";
import LineString from "ol/geom/LineString";
import { toLonLat, fromLonLat } from "ol/proj";
import type { EventsKey } from "ol/events";
import { domainAccent, type DomainMode } from "@/lib/tokens";

export interface ProfileLine {
  start: [number, number]; // lon, lat
  end: [number, number]; // lon, lat
}

interface UseProfileToolArgs {
  map: Map | null;
  active: boolean;
  domain: DomainMode;
}

export function useProfileTool({ map, active, domain }: UseProfileToolArgs) {
  const [line, setLine] = useState<ProfileLine | null>(null);
  const sourceRef = useRef<VectorSource | null>(null);
  const layerRef = useRef<VectorLayer | null>(null);
  const clickKeyRef = useRef<EventsKey | null>(null);
  const firstPointRef = useRef<[number, number] | null>(null);

  // Set up the profile layer once
  useEffect(() => {
    if (!map) return;
    const source = new VectorSource();
    const layer = new VectorLayer({
      source,
      style: new Style({
        stroke: new Stroke({
          color: domainAccent[domain].primary,
          width: 2,
          lineDash: [6, 3],
        }),
      }),
    });
    map.addLayer(layer);
    sourceRef.current = source;
    layerRef.current = layer;
    return () => {
      map.removeLayer(layer);
      sourceRef.current = null;
      layerRef.current = null;
    };
  }, [map, domain]);

  // Wire click handler when active
  useEffect(() => {
    const source = sourceRef.current;
    if (!map || !source) return;
    if (!active) {
      // Clean up: clear layer + reset state
      source.clear();
      firstPointRef.current = null;
      setLine(null);
      return;
    }

    const onClick = (evt: { coordinate: number[] }) => {
      const coord = evt.coordinate;
      const lonLat = toLonLat(coord) as [number, number];
      if (!firstPointRef.current) {
        firstPointRef.current = lonLat;
        source.clear();
      } else {
        const start = firstPointRef.current;
        const end = lonLat;
        source.clear();
        const proj = map.getView().getProjection();
        const feature = new Feature({
          geometry: new LineString([
            fromLonLat(start, proj),
            fromLonLat(end, proj),
          ]),
        });
        source.addFeature(feature);
        setLine({ start, end });
        firstPointRef.current = null;
      }
    };

    // OL's on() returns an EventsKey we can pass to un()
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const key = map.on("click", onClick as any) as EventsKey;
    clickKeyRef.current = key;

    return () => {
      if (clickKeyRef.current) {
        map.un("click", onClick as never);
        clickKeyRef.current = null;
      }
    };
  }, [map, active]);

  return {
    line,
    clear: () => {
      sourceRef.current?.clear();
      firstPointRef.current = null;
      setLine(null);
    },
  };
}
