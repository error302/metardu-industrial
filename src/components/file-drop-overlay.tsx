/**
 * File drop zone overlay — covers the map canvas when the user drags
 * files over the workspace. Detects survey file types by extension
 * and shows the right kind hint.
 *
 * Per ARCHITECTURE.md §6.6 / §6.7 — the surveyor's primary interaction
 * is dropping raw sensor files (LAS/GeoTIFF/.all) onto the canvas.
 */

import { useState, type DragEvent } from "react";
import { UploadCloud, FileBox, AlertCircle } from "lucide-react";
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

export function FileDropOverlay({ domain }: Props) {
  const [isDragging, setIsDragging] = useState(false);
  const [rejected, setRejected] = useState<string[]>([]);
  const addFile = useSurveyStore((s) => s.addFile);

  function onDragOver(e: DragEvent) {
    e.preventDefault();
    setIsDragging(true);
  }

  function onDragLeave(e: DragEvent) {
    e.preventDefault();
    setIsDragging(false);
  }

  function onDrop(e: DragEvent) {
    e.preventDefault();
    setIsDragging(false);
    setRejected([]);

    const files = Array.from(e.dataTransfer.files);
    const accepted: File[] = [];
    const rej: string[] = [];

    for (const file of files) {
      const lower = file.name.toLowerCase();
      const ok =
        lower.endsWith(".las") ||
        lower.endsWith(".laz") ||
        lower.endsWith(".tif") ||
        lower.endsWith(".tiff") ||
        lower.endsWith(".all") ||
        lower.endsWith(".s7k") ||
        lower.endsWith(".bsf") ||
        lower.endsWith(".csv") ||
        lower.endsWith(".tsv") ||
        lower.endsWith(".gpkg") ||
        lower.endsWith(".kml") ||
        lower.endsWith(".mrk") ||
        lower.endsWith(".json");
      if (ok) accepted.push(file);
      else rej.push(file.name);
    }

    for (const file of accepted) addFile(file);
    if (rej.length) setRejected(rej);
  }

  if (!isDragging && rejected.length === 0) {
    // Invisible catcher while not dragging — covers the canvas so the drop
    // event has something to land on. Pointer-events only on dragenter.
    return (
      <div
        onDragOver={onDragOver}
        onDrop={onDrop}
        className="absolute inset-0 z-30"
        style={{ pointerEvents: "none" }}
        aria-hidden
      />
    );
  }

  const accent = domainAccent[domain].primary;

  return (
    <div
      onDragOver={onDragOver}
      onDragLeave={onDragLeave}
      onDrop={onDrop}
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
