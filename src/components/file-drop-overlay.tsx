/**
 * File drop zone overlay — covers the map canvas when the user drags
 * files over the workspace. Detects survey file types by extension
 * and shows the right kind hint.
 *
 * Per ARCHITECTURE.md §6.6 / §6.7 — the surveyor's primary interaction
 * is dropping raw sensor files (LAS/GeoTIFF/.all) onto the canvas.
 */

import { useEffect, useState } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { UploadCloud, FileBox, AlertCircle } from "lucide-react";
import { isNative } from "@/lib/tauri-ipc";
import { colors, domainAccent, type DomainMode } from "@/lib/tokens";
import { useSurveyStore, type SurveyFileKind } from "@/stores/survey-store";

const KIND_LABEL: Record<SurveyFileKind, string> = {
  las: "LAS/LAZ point cloud",
  geotiff: "GeoTIFF raster",
  "mbes-all": "Kongsberg .all (MbES)",
  "mbes-s7k": "Reson .s7k (MbES)",
  "mbes-bsf": "R2Sonic .bsf (MbES)",
  csv: "Tabular / drone manifest CSV",
  geopkg: "GeoPackage vector",
  kml: "KML",
  "drone-mrk": "DJI MMC drone manifest",
  "drone-json": "DJI FlightHub JSON",
  unknown: "Unknown format",
};

interface Props {
  domain: DomainMode;
}

function acceptsFile(path: string): boolean {
  return /\.(las|laz|tif|tiff|all|s7k|bsf|csv|tsv|gpkg|kml|mrk|json)$/i.test(path);
}

export function FileDropOverlay({ domain }: Props) {
  const [isDragging, setIsDragging] = useState(false);
  const [rejected, setRejected] = useState<string[]>([]);
  const addFile = useSurveyStore((s) => s.addFile);
  const addFileFromPath = useSurveyStore((s) => s.addFileFromPath);

  useEffect(() => {
    if (!isNative()) return;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void getCurrentWebview().onDragDropEvent(({ payload }) => {
      if (payload.type === "enter") setIsDragging(true);
      if (payload.type === "leave") setIsDragging(false);
      if (payload.type !== "drop") return;
      setIsDragging(false);
      const rejectedPaths: string[] = [];
      for (const path of payload.paths) {
        if (acceptsFile(path)) addFileFromPath(path, 0);
        else rejectedPaths.push(path.split(/[\\/]/).pop() ?? path);
      }
      setRejected(rejectedPaths);
    }).then((stop) => {
      if (disposed) stop();
      else unlisten = stop;
    }).catch((error: unknown) => {
      console.error("Unable to listen for native file drops:", error);
      if (!disposed) setRejected(["Native file drop is unavailable. Restart the application and try again."]);
    });
    return () => { disposed = true; unlisten?.(); };
  }, [addFileFromPath]);

  useEffect(() => {
  if (isNative()) return;
  function onDragOver(e: globalThis.DragEvent) {
    if (!e.dataTransfer?.types.includes("Files")) return;
    e.preventDefault();
    setIsDragging(true);
  }

  function onDragLeave(e: globalThis.DragEvent) {
    if (!e.relatedTarget) setIsDragging(false);
  }

  function onDrop(e: globalThis.DragEvent) {
    if (!e.dataTransfer?.types.includes("Files")) return;
    e.preventDefault();
    setIsDragging(false);
    setRejected([]);

    const files = Array.from(e.dataTransfer.files);
    const accepted: File[] = [];
    const rej: string[] = [];

    for (const file of files) {
      if (acceptsFile(file.name)) accepted.push(file);
      else rej.push(file.name);
    }

    for (const file of accepted) addFile(file);
    if (rej.length) setRejected(rej);
  }

  window.addEventListener("dragover", onDragOver);
  window.addEventListener("dragleave", onDragLeave);
  window.addEventListener("drop", onDrop);
  return () => {
    window.removeEventListener("dragover", onDragOver);
    window.removeEventListener("dragleave", onDragLeave);
    window.removeEventListener("drop", onDrop);
  };
  }, [addFile]);

  if (!isDragging && rejected.length === 0) {
    return null;
  }

  const accent = domainAccent[domain].primary;

  return (
    <div
      className="absolute inset-0 z-40 flex items-center justify-center backdrop-blur-sm"
      style={{ background: "rgba(10, 25, 47, 0.85)" }}
    >
      <div
        className="rounded-lg border-2 border-dashed p-10 text-center"
        style={{
          borderColor: accent,
          background: `${accent}10`,
          minWidth: 360,
        }}
      >
        <UploadCloud
          className="mx-auto h-12 w-12"
          style={{ color: accent }}
        />
        <h3 className="mt-3 text-lg font-semibold text-white">
          Drop survey files to ingest
        </h3>
        <p className="mt-1 text-xs text-steel-light">
          {domainAccent[domain].label} mode · supported formats:
        </p>
        <div className="mt-3 flex flex-wrap justify-center gap-1.5">
          {Object.entries(KIND_LABEL).map(([k, label]) => (
            <span
              key={k}
              className="rounded-sm border px-2 py-0.5 text-[10px] font-mono"
              style={{
                borderColor: `${accent}40`,
                color: colors.steelLight,
                background: `${accent}08`,
              }}
            >
              {label}
            </span>
          ))}
        </div>

        {rejected.length > 0 && (
          <div
            className="mt-4 rounded-md border p-3 text-left text-xs"
            style={{ borderColor: `${colors.fail}40`, background: `${colors.fail}10` }}
          >
            <div
              className="mb-1.5 flex items-center gap-1.5 font-semibold"
              style={{ color: colors.fail }}
            >
              <AlertCircle className="h-3.5 w-3.5" />
              Unsupported file{rejected.length > 1 ? "s" : ""}
            </div>
            <ul className="space-y-0.5 font-mono text-[10px] text-steel-light">
              {rejected.map((n) => (
                <li key={n}>{n}</li>
              ))}
            </ul>
            <button type="button" onClick={() => setRejected([])} className="mt-3 rounded border px-2 py-1 text-steel-light hover:text-white">Dismiss</button>
          </div>
        )}

        <div className="mt-4 flex items-center justify-center gap-2 text-[10px] text-steel-gray">
          <FileBox className="h-3 w-3" />
          Files stay on disk — MetaRDU reads them in place
        </div>
      </div>
    </div>
  );
}
