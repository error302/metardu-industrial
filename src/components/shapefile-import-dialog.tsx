/**
 * Shapefile Import Dialog — Sprint 16.
 *
 * Frontend for the `read_shapefile_cmd` IPC command. Lets the surveyor
 * load an ESRI Shapefile (.shp + .shx + .dbf) and inspect its contents:
 * shape type, feature count, bounds, attribute columns, and per-feature
 * geometry + attributes.
 *
 * Activated the Spatial Data Engineer agent methodology for ETL workflows.
 */

import { useState } from "react";
import { FileSearch, Loader2, Table2, Map as MapIcon } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { colors } from "@/lib/tokens";
import { isNative } from "@/lib/tauri-ipc";
import { DialogShell, DialogButton, EmptyState } from "@/components/dialog-shell";
import { FileInput } from "@/components/file-input";

type ShapeType = "null" | "point" | "polyline" | "polygon" | "multipoint";

interface ShapefileFeature {
  geometry: { kind: string; x?: number; y?: number; parts?: number[][][]; rings?: number[][][]; points?: number[][] };
  attributes: Record<string, string>;
}

interface Shapefile {
  shape_type: ShapeType;
  features: ShapefileFeature[];
  bounds: [number, number, number, number];
}

interface Props {
  open: boolean;
  onClose: () => void;
}

const SHAPE_TYPE_LABELS: Record<ShapeType, string> = {
  null: "Null",
  point: "Point",
  polyline: "Polyline",
  polygon: "Polygon",
  multipoint: "MultiPoint",
};

export function ShapefileImportDialog({ open, onClose }: Props) {
  const [filePath, setFilePath] = useState("");
  const [shapefile, setShapefile] = useState<Shapefile | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selectedFeature, setSelectedFeature] = useState<number | null>(null);

  async function handleLoad() {
    setLoading(true);
    setError(null);
    setShapefile(null);
    setSelectedFeature(null);
    try {
      if (!isNative()) {
        setError("Browser mode — Shapefile import requires the native Tauri shell");
        return;
      }
      const result = await invoke<Shapefile>("read_shapefile_cmd", { path: filePath });
      setShapefile(result);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }

  // Collect all attribute keys
  const attributeKeys = shapefile
    ? Array.from(new Set(shapefile.features.flatMap((f) => Object.keys(f.attributes))))
    : [];

  return (
    <DialogShell
      open={open}
      onClose={onClose}
      title="Shapefile Import"
      icon={<FileSearch className="h-4 w-4" />}
      iconColor={colors.accent}
      maxWidth="max-w-4xl"
      subtitle={shapefile ? `${shapefile.features.length} features · ${SHAPE_TYPE_LABELS[shapefile.shape_type]}` : "ESRI Shapefile (.shp/.shx/.dbf)"}
      footerHint="The #1 interchange format for mine plans and engineering drawings"
      actions={
        <>
          <DialogButton variant="secondary" onClick={onClose}>Close</DialogButton>
          <DialogButton
            variant="primary"
            onClick={handleLoad}
            disabled={loading || !filePath.trim()}
          >
            {loading ? <Loader2 className="h-3 w-3 animate-spin" /> : <FileSearch className="h-3 w-3" />}
            {loading ? "Loading…" : "Import"}
          </DialogButton>
        </>
      }
    >
      <div className="space-y-4">
        {/* File input */}
        <FileInput
          value={filePath}
          onChange={setFilePath}
          extensions={["shp"]}
          filterName="ESRI Shapefile"
          storageKey="shapefile-import"
          placeholder="/path/to/stockpads.shp"
        />

        {error && (
          <div className="rounded-md border p-3 text-xs" style={{ borderColor: `${colors.fail}40`, background: `${colors.fail}10`, color: colors.fail }}>
            {error}
          </div>
        )}

        {shapefile && (
          <>
            {/* Summary stats */}
            <div className="grid grid-cols-4 gap-2">
              <Kpi label="Shape Type" value={SHAPE_TYPE_LABELS[shapefile.shape_type]} color={colors.accent} />
              <Kpi label="Features" value={shapefile.features.length.toLocaleString()} color={colors.steelLight} />
              <Kpi label="Attributes" value={attributeKeys.length.toString()} color={colors.steelLight} />
              <Kpi label="Bounds" value={`${shapefile.bounds[0].toFixed(0)},${shapefile.bounds[1].toFixed(0)} → ${shapefile.bounds[2].toFixed(0)},${shapefile.bounds[3].toFixed(0)}`} color={colors.steelLight} />
            </div>

            {/* Feature table */}
            <div>
              <div className="mb-1.5 flex items-center gap-1.5 text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
                <Table2 className="h-3 w-3" /> Feature Table ({shapefile.features.length} features)
              </div>
              <div className="max-h-48 overflow-auto rounded-md border border-navy-border">
                <table className="table-enterprise w-full text-left text-[10px]">
                  <thead className="sticky top-0 bg-navy-elevated text-steel-gray">
                    <tr>
                      <th className="px-2 py-1.5">#</th>
                      {attributeKeys.map((k) => (
                        <th key={k} className="px-2 py-1.5">{k}</th>
                      ))}
                    </tr>
                  </thead>
                  <tbody>
                    {shapefile.features.slice(0, 200).map((f, i) => (
                      <tr
                        key={i}
                        onClick={() => setSelectedFeature(i)}
                        className={`cursor-pointer border-t border-navy-border ${selectedFeature === i ? "bg-navy-elevated" : "hover:bg-navy-elevated/50"}`}
                      >
                        <td className="px-2 py-1 font-mono text-steel-gray">{i + 1}</td>
                        {attributeKeys.map((k) => (
                          <td key={k} className="px-2 py-1 font-mono text-steel-light">
                            {f.attributes[k] ?? "—"}
                          </td>
                        ))}
                      </tr>
                    ))}
                  </tbody>
                </table>
                {shapefile.features.length > 200 && (
                  <div className="border-t border-navy-border p-1 text-center text-[9px] text-steel-gray">
                    +{(shapefile.features.length - 200).toLocaleString()} more features
                  </div>
                )}
              </div>
            </div>

            {/* Selected feature detail */}
            {selectedFeature != null && shapefile.features[selectedFeature] && (
              <div className="input-enterprise rounded-md border border-navy-border bg-navy-base p-3">
                <div className="mb-1.5 flex items-center gap-1.5 text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
                  <MapIcon className="h-3 w-3" /> Feature {selectedFeature + 1} Detail
                </div>
                <pre className="overflow-auto font-mono text-[10px] text-steel-light">
{JSON.stringify(shapefile.features[selectedFeature], null, 2)}
                </pre>
              </div>
            )}
          </>
        )}

        {!shapefile && !loading && !error && (
          <EmptyState
            icon={<FileSearch className="h-8 w-8" />}
            title="No Shapefile loaded"
            description="Browse for a .shp file to import. The .shx and .dbf files in the same directory are loaded automatically."
          />
        )}
      </div>
    </DialogShell>
  );
}

function Kpi({ label, value, color }: { label: string; value: string; color: string }) {
  return (
    <div className="card-enterprise rounded-md border p-2" style={{ borderColor: `${color}40`, background: `${color}10` }}>
      <div className="text-[9px] uppercase tracking-wider" style={{ color }}>{label}</div>
      <div className="mt-0.5 font-mono text-[11px] font-bold text-white truncate">{value}</div>
    </div>
  );
}
