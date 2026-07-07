/**
 * Map Layout Dialog — Sprint 16/17 frontend for generate_map_layout_cmd.
 *
 * Captures the current OpenLayers map canvas as a PNG, then sends it to
 * the Rust backend with layout parameters (title, surveyor, date, scale,
 * CRS, legend, page size) to generate a print-quality map sheet PDF.
 *
 * The PDF includes:
 *   - Title block (project, surveyor, date, scale, CRS)
 *   - North arrow
 *   - Scale bar
 *   - Coordinate grid labels (corner coordinates)
 *   - Legend (color swatches + labels)
 *   - Border
 *   - Footer with generation timestamp
 */

import { useState, useRef } from "react";
import { Map as MapIcon, Loader2, Download, FileDown } from "lucide-react";
import type Map from "ol/Map";
import { invoke } from "@tauri-apps/api/core";
import { colors } from "@/lib/tokens";
import { isNative } from "@/lib/tauri-ipc";
import { DialogShell, DialogButton } from "@/components/dialog-shell";
import { FileInput } from "@/components/file-input";

interface MapLayoutRequest {
  output_path: string;
  map_image_base64: string;
  map_width_px: number;
  map_height_px: number;
  page_size: string;
  orientation: string;
  project_name: string;
  surveyor: string;
  survey_date: string;
  scale: string;
  crs: string;
  legend: [string, string][];
  north_rotation_deg: number;
  bounds: [number, number, number, number] | null;
}

interface MapLayoutResult {
  path: string;
  file_size_bytes: number;
}

interface LegendEntry {
  color: string;
  label: string;
}

interface Props {
  open: boolean;
  onClose: () => void;
  map: Map | null;
  /** Default values from the current project */
  defaultProjectName?: string;
  defaultCrs?: string;
}

export function MapLayoutDialog({ open, onClose, map, defaultProjectName = "", defaultCrs = "EPSG:3857" }: Props) {
  const [outputPath, setOutputPath] = useState("");
  const [pageSize, setPageSize] = useState("a3");
  const [orientation, setOrientation] = useState("landscape");
  const [projectName, setProjectName] = useState(defaultProjectName);
  const [surveyor, setSurveyor] = useState("");
  const [surveyDate, setSurveyDate] = useState(new Date().toISOString().slice(0, 10));
  const [scale, setScale] = useState("1:1000");
  const [crs, setCrs] = useState(defaultCrs);
  const [legend, setLegend] = useState<LegendEntry[]>([
    { color: "#22C55E", label: "Fill (added)" },
    { color: "#EF4444", label: "Cut (removed)" },
  ]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<MapLayoutResult | null>(null);
  const canvasRef = useRef<HTMLCanvasElement | null>(null);

  function captureMapCanvas(): { base64: string; width: number; height: number } | null {
    if (!map) return null;

    // OL renders to a canvas in the map viewport
    const viewport = map.getTargetElement();
    const canvas = viewport.querySelector("canvas");
    if (!canvas) {
      setError("No canvas found in map viewport — try moving the map first");
      return null;
    }

    // Create a new canvas at higher resolution for print quality (2x)
    const scale = 2;
    const printCanvas = document.createElement("canvas");
    printCanvas.width = canvas.width * scale;
    printCanvas.height = canvas.height * scale;
    const ctx = printCanvas.getContext("2d");
    if (!ctx) {
      setError("Canvas 2D context unavailable");
      return null;
    }
    ctx.drawImage(canvas, 0, 0, printCanvas.width, printCanvas.height);

    const dataUrl = printCanvas.toDataURL("image/png");
    canvasRef.current = printCanvas;

    return {
      base64: dataUrl,
      width: printCanvas.width,
      height: printCanvas.height,
    };
  }

  function getMapBounds(): [number, number, number, number] | null {
    if (!map) return null;
    const extent = map.getView().calculateExtent(map.getSize());
    return [extent[0], extent[1], extent[2], extent[3]];
  }

  async function handleGenerate() {
    setLoading(true);
    setError(null);
    setResult(null);
    try {
      if (!isNative()) {
        setError("Browser mode — PDF generation requires the native Tauri shell");
        return;
      }
      if (!outputPath.trim()) {
        setError("Enter an output PDF path");
        return;
      }

      const captured = captureMapCanvas();
      if (!captured) return;

      const bounds = getMapBounds();
      const northRotation = (map?.getView().getRotation() || 0) * (180 / Math.PI);

      const request: MapLayoutRequest = {
        output_path: outputPath,
        map_image_base64: captured.base64,
        map_width_px: captured.width,
        map_height_px: captured.height,
        page_size: pageSize,
        orientation,
        project_name: projectName,
        surveyor,
        survey_date: surveyDate,
        scale,
        crs,
        legend: legend.map((e) => [e.color, e.label]),
        north_rotation_deg: northRotation,
        bounds,
      };

      const result = await invoke<MapLayoutResult>("generate_map_layout_cmd", { request });
      setResult(result);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }

  function addLegendEntry() {
    setLegend([...legend, { color: "#94A3B8", label: "" }]);
  }

  function removeLegendEntry(i: number) {
    setLegend(legend.filter((_, idx) => idx !== i));
  }

  function updateLegendEntry(i: number, patch: Partial<LegendEntry>) {
    setLegend(legend.map((e, idx) => (idx === i ? { ...e, ...patch } : e)));
  }

  return (
    <DialogShell
      open={open}
      onClose={onClose}
      title="Generate Map Sheet"
      icon={<MapIcon className="h-4 w-4" />}
      iconColor={colors.accent}
      maxWidth="max-w-2xl"
      subtitle="Print-quality PDF with title block, north arrow, scale bar, legend"
      footerHint="Captures the current map view at 2× resolution for print quality"
      actions={
        <>
          <DialogButton variant="secondary" onClick={onClose}>Close</DialogButton>
          <DialogButton
            variant="primary"
            onClick={handleGenerate}
            disabled={loading || !outputPath.trim() || !map}
          >
            {loading ? <Loader2 className="h-3 w-3 animate-spin" /> : <FileDown className="h-3 w-3" />}
            {loading ? "Generating…" : "Generate PDF"}
          </DialogButton>
        </>
      }
    >
      <div className="space-y-4">
        {/* Output path */}
        <div>
          <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
            Output PDF path
          </label>
          <FileInput
            value={outputPath}
            onChange={setOutputPath}
            save
            extensions={["pdf"]}
            filterName="PDF Document"
            storageKey="map-layout-output"
            placeholder="/path/to/map_sheet.pdf"
          />
        </div>

        {/* Page setup */}
        <div className="grid grid-cols-2 gap-3">
          <div>
            <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">Page size</label>
            <select
              value={pageSize}
              onChange={(e) => setPageSize(e.target.value)}
              className="w-full rounded-md border border-navy-border bg-navy-base px-2 py-1.5 text-sm text-white"
            >
              <option value="a3">A3 (420×297mm)</option>
              <option value="a4">A4 (297×210mm)</option>
              <option value="letter">Letter (11×8.5in)</option>
            </select>
          </div>
          <div>
            <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">Orientation</label>
            <div className="flex gap-1">
              <button
                onClick={() => setOrientation("landscape")}
                className={`flex-1 rounded-md px-3 py-1.5 text-xs font-medium ${orientation === "landscape" ? "text-navy-base" : "text-steel-gray"}`}
                style={{ background: orientation === "landscape" ? colors.accent : colors.navyBase, border: `1px solid ${colors.accent}40` }}
              >
                Landscape
              </button>
              <button
                onClick={() => setOrientation("portrait")}
                className={`flex-1 rounded-md px-3 py-1.5 text-xs font-medium ${orientation === "portrait" ? "text-navy-base" : "text-steel-gray"}`}
                style={{ background: orientation === "portrait" ? colors.accent : colors.navyBase, border: `1px solid ${colors.accent}40` }}
              >
                Portrait
              </button>
            </div>
          </div>
        </div>

        {/* Title block fields */}
        <div className="grid grid-cols-2 gap-3">
          <div>
            <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">Project name</label>
            <input
              type="text"
              value={projectName}
              onChange={(e) => setProjectName(e.target.value)}
              placeholder="Stockpile Audit — Pad A"
              className="w-full rounded-md border border-navy-border bg-navy-base px-2 py-1.5 text-xs text-white focus:border-accent focus:outline-none"
            />
          </div>
          <div>
            <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">Surveyor</label>
            <input
              type="text"
              value={surveyor}
              onChange={(e) => setSurveyor(e.target.value)}
              placeholder="Surveyor name"
              className="w-full rounded-md border border-navy-border bg-navy-base px-2 py-1.5 text-xs text-white focus:border-accent focus:outline-none"
            />
          </div>
          <div>
            <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">Survey date</label>
            <input
              type="date"
              value={surveyDate}
              onChange={(e) => setSurveyDate(e.target.value)}
              className="w-full rounded-md border border-navy-border bg-navy-base px-2 py-1.5 font-mono text-xs text-white focus:border-accent focus:outline-none"
            />
          </div>
          <div>
            <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">Scale</label>
            <input
              type="text"
              value={scale}
              onChange={(e) => setScale(e.target.value)}
              placeholder="1:1000"
              className="w-full rounded-md border border-navy-border bg-navy-base px-2 py-1.5 font-mono text-xs text-white focus:border-accent focus:outline-none"
            />
          </div>
        </div>

        {/* CRS */}
        <div>
          <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">CRS</label>
          <input
            type="text"
            value={crs}
            onChange={(e) => setCrs(e.target.value)}
            placeholder="EPSG:28355"
            className="w-full rounded-md border border-navy-border bg-navy-base px-2 py-1.5 font-mono text-xs text-white focus:border-accent focus:outline-none"
          />
        </div>

        {/* Legend */}
        <div>
          <div className="mb-1.5 flex items-center justify-between">
            <label className="text-[10px] font-semibold uppercase tracking-wider text-steel-gray">Legend</label>
            <button
              onClick={addLegendEntry}
              className="rounded px-2 py-0.5 text-[10px] font-medium"
              style={{ background: colors.steelLight, color: colors.navyBase }}
            >
              + Add entry
            </button>
          </div>
          <div className="space-y-1">
            {legend.map((entry, i) => (
              <div key={i} className="flex items-center gap-2">
                <input
                  type="color"
                  value={entry.color}
                  onChange={(e) => updateLegendEntry(i, { color: e.target.value })}
                  className="h-7 w-10 rounded border border-navy-border bg-navy-base cursor-pointer"
                />
                <input
                  type="text"
                  value={entry.label}
                  onChange={(e) => updateLegendEntry(i, { label: e.target.value })}
                  placeholder="Legend label"
                  className="flex-1 rounded-md border border-navy-border bg-navy-base px-2 py-1 text-xs text-white focus:border-accent focus:outline-none"
                />
                <button
                  onClick={() => removeLegendEntry(i)}
                  className="rounded px-2 py-1 text-[10px] text-fail hover:bg-fail/20"
                >
                  ✕
                </button>
              </div>
            ))}
          </div>
        </div>

        {error && (
          <div className="rounded-md border p-3 text-xs" style={{ borderColor: `${colors.fail}40`, background: `${colors.fail}10`, color: colors.fail }}>
            {error}
          </div>
        )}

        {result && (
          <div className="flex items-center gap-2 rounded-md border p-3" style={{ borderColor: `${colors.pass}40`, background: `${colors.pass}10` }}>
            <Download className="h-4 w-4" style={{ color: colors.pass }} />
            <div className="flex-1">
              <div className="text-sm font-semibold text-white">Map sheet generated</div>
              <div className="text-[10px] font-mono text-steel-light">{result.path}</div>
              <div className="text-[10px] text-steel-gray">{(result.file_size_bytes / 1024).toFixed(1)} KB</div>
            </div>
          </div>
        )}

        {!map && (
          <div className="rounded-md border p-2 text-[10px]" style={{ borderColor: `${colors.warn}40`, background: `${colors.warn}10`, color: colors.warn }}>
            No map loaded — open a survey file first to capture the map canvas.
          </div>
        )}
      </div>
    </DialogShell>
  );
}
