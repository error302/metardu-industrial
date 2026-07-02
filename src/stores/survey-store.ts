/**
 * Survey ingest store — tracks dropped/opened survey files.
 *
 * Phase 0: just tracks the file metadata so the UI can react.
 * Phase 1+: invokes Rust core to actually parse and render the data.
 */

import { create } from "zustand";

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

export interface SurveyFile {
  id: string;
  name: string;
  path: string;
  size: number;
  kind: SurveyFileKind;
  addedAt: number;
  status: "pending" | "loading" | "loaded" | "error";
  errorMessage?: string;
}

interface SurveyState {
  files: SurveyFile[];
  activeFileId: string | null;
  addFile: (file: File) => string;
  addFileFromPath: (path: string, size: number) => string;
  removeFile: (id: string) => void;
  setActiveFile: (id: string | null) => void;
  updateFileStatus: (
    id: string,
    status: SurveyFile["status"],
    errorMessage?: string,
  ) => void;
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

export const useSurveyStore = create<SurveyState>((set) => ({
  files: [],
  activeFileId: null,

  addFile: (file) => {
    const id = makeId();
    // Browser File objects don't expose .path; Tauri drag-drop events
    // provide paths separately and call addFileFromPath instead.
    const path =
      typeof file === "object" && file !== null && "path" in file
        ? String((file as { path?: string }).path ?? file.name)
        : file.name;
    const survey: SurveyFile = {
      id,
      name: file.name,
      path,
      size: file.size,
      kind: classifyByExt(file.name),
      addedAt: Date.now(),
      status: "pending",
    };
    set((s) => ({ files: [...s.files, survey], activeFileId: id }));
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

  clear: () => set({ files: [], activeFileId: null }),
}));
