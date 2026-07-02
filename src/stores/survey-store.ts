/**
 * Survey ingest store — tracks dropped/opened survey files.
 *
 * Phase 0:
 *   - Tracks file metadata so the UI can react
 *   - Calls Rust probe_file() to get real bounds/header info
 *   - Stores bounds for rendering on the OpenLayers canvas
 * Phase 1+:
 *   - Triggers full ingest pipeline (classification, CUBE, etc.)
 */

import { create } from "zustand";
import {
  probeFile,
  type FileProbeResult,
  type GeoTiffHeaderRpc,
  type KongsbergAllHeaderRpc,
  type LasHeaderRpc,
} from "@/lib/tauri-ipc";

export type SurveyFileKind =
  | "las" // LAS/LAZ point cloud
  | "geotiff" // raster DEM / orthomosaic
  | "mbes-all" // Kongsberg .all
  | "mbes-s7k" // Reson .s7k
  | "mbes-bsf" // R2Sonic .bsf
  | "csv" // tabular fix lists / control points
  | "geopkg" // GeoPackage vector
  | "kml"
  | "unknown";

export interface Bounds {
  min_x: number;
  min_y: number;
  max_x: number;
  max_y: number;
}

export interface SurveyFile {
  id: string;
  name: string;
  path: string;
  size: number;
  kind: SurveyFileKind;
  addedAt: number;
  status: "pending" | "probing" | "loaded" | "error";
  errorMessage?: string;
  // Populated after probe
  bounds?: Bounds;
  pointCount?: number;
  lasVersion?: string;
  pdrf?: number;
  crsWkt?: string | null;
  epsg?: number | null; // EPSG code extracted from GeoTIFF GeoKeyDirectory
  dimensions?: { width: number; height: number }; // for rasters
  vendor?: string;
}

interface SurveyState {
  files: SurveyFile[];
  activeFileId: string | null;
  /** Most recent file with an EPSG code — used for auto-CRS-switch prompts */
  lastDetectedEpsg: string | null;
  addFile: (file: File) => string;
  addFileFromPath: (path: string, size: number) => string;
  removeFile: (id: string) => void;
  setActiveFile: (id: string | null) => void;
  updateFileStatus: (
    id: string,
    status: SurveyFile["status"],
    errorMessage?: string,
  ) => void;
  probeFile: (id: string) => Promise<void>;
  clear: () => void;
}

function classifyByExt(name: string): SurveyFileKind {
  const lower = name.toLowerCase();
  if (lower.endsWith(".las") || lower.endsWith(".laz")) return "las";
  if (lower.endsWith(".tif") || lower.endsWith(".tiff")) return "geotiff";
  if (lower.endsWith(".all")) return "mbes-all";
  if (lower.endsWith(".s7k")) return "mbes-s7k";
  if (lower.endsWith(".bsf")) return "mbes-bsf";
  if (lower.endsWith(".csv") || lower.endsWith(".tsv")) return "csv";
  if (lower.endsWith(".gpkg") || lower.endsWith(".geopkg")) return "geopkg";
  if (lower.endsWith(".kml")) return "kml";
  return "unknown";
}

function makeId(): string {
  return `sf_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 8)}`;
}

/** Extract path from a browser File (Tauri provides it; browsers don't). */
function extractPath(file: File): string {
  if (typeof file === "object" && file !== null && "path" in file) {
    return String((file as { path?: string }).path ?? file.name);
  }
  return file.name;
}

export const useSurveyStore = create<SurveyState>((set, get) => ({
  files: [],
  activeFileId: null,
  lastDetectedEpsg: null,

  addFile: (file) => {
    const id = makeId();
    const survey: SurveyFile = {
      id,
      name: file.name,
      path: extractPath(file),
      size: file.size,
      kind: classifyByExt(file.name),
      addedAt: Date.now(),
      status: "pending",
    };
    set((s) => ({ files: [...s.files, survey], activeFileId: id }));
    // Kick off probe immediately
    void get().probeFile(id);
    return id;
  },

  addFileFromPath: (path, size) => {
    const id = makeId();
    const name = path.split(/[\\/]/).pop() ?? path;
    const survey: SurveyFile = {
      id,
      name,
      path,
      size,
      kind: classifyByExt(name),
      addedAt: Date.now(),
      status: "pending",
    };
    set((s) => ({ files: [...s.files, survey], activeFileId: id }));
    void get().probeFile(id);
    return id;
  },

  removeFile: (id) =>
    set((s) => ({
      files: s.files.filter((f) => f.id !== id),
      activeFileId: s.activeFileId === id ? null : s.activeFileId,
    })),

  setActiveFile: (id) => set({ activeFileId: id }),

  updateFileStatus: (id, status, errorMessage) =>
    set((s) => ({
      files: s.files.map((f) =>
        f.id === id ? { ...f, status, errorMessage } : f,
      ),
    })),

  probeFile: async (id) => {
    const file = get().files.find((f) => f.id === id);
    if (!file) return;

    set((s) => ({
      files: s.files.map((f) =>
        f.id === id ? { ...f, status: "probing" } : f,
      ),
    }));

    try {
      const result: FileProbeResult = await probeFile(file.path);
      set((s) => ({
        files: s.files.map((f) => {
          if (f.id !== id) return f;
          const updated: SurveyFile = { ...f, status: "loaded" };
          if (result.kind === "las") {
            const h: LasHeaderRpc = result.header;
            updated.bounds = {
              min_x: h.min_x,
              min_y: h.min_y,
              max_x: h.max_x,
              max_y: h.max_y,
            };
            updated.pointCount = h.point_count;
            updated.lasVersion = `${h.version_major}.${h.version_minor}`;
            updated.pdrf = h.point_data_format;
            updated.crsWkt = h.crs_wkt;
          } else if (result.kind === "geo-tiff") {
            const h: GeoTiffHeaderRpc = result.header;
            if (h.bounds) {
              updated.bounds = {
                min_x: h.bounds[0],
                min_y: h.bounds[1],
                max_x: h.bounds[2],
                max_y: h.bounds[3],
              };
            }
            updated.epsg = h.epsg;
            updated.dimensions = {
              width: h.width,
              height: h.length,
            };
          } else if (result.kind === "kongsberg-all") {
            const h: KongsbergAllHeaderRpc = result.header;
            updated.vendor = h.model;
            updated.pointCount = h.ping_count;
            // Kongsberg .all doesn't carry geographic bounds in the
            // header — they emerge after parsing position datagrams,
            // which is Phase 2 work. For now we leave bounds undefined.
          } else if (result.kind === "mb-es") {
            updated.vendor = result.vendor;
            updated.size = result.size_bytes;
          }
          return updated;
        }),
        // Track most recent EPSG detection for auto-CRS-switch prompts
        lastDetectedEpsg:
          result.kind === "geo-tiff" && result.header.epsg
            ? `EPSG:${result.header.epsg}`
            : get().lastDetectedEpsg,
      }));
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      set((s) => ({
        files: s.files.map((f) =>
          f.id === id
            ? { ...f, status: "error", errorMessage: msg }
            : f,
        ),
      }));
    }
  },

  clear: () => set({ files: [], activeFileId: null, lastDetectedEpsg: null }),
}));
