/**
 * S-57 Export Dialog — Phase 2 Marine MVP.
 *
 * Digitize marine features (wrecks, obstructions, rocks) and export them
 * to an S-57 .000 file. Features can be added manually via coordinate
 * input or from a CSV. The exported file can be ingested by CARIS S-57
 * Composer or any S-57 reader.
 */

import { useState } from "react";
import { X, Download, Plus, Trash2, Anchor } from "lucide-react";
import { colors } from "@/lib/tokens";
import {
  exportS57,
  type S57Attribute,
  type S57Feature,
  type S57Geometry,
  type S57ObjectClass,
} from "@/lib/tauri-ipc";

interface Props {
  open: boolean;
  onClose: () => void;
}

const OBJECT_CLASSES: { value: S57ObjectClass; label: string }[] = [
  { value: "WRECKS", label: "WRECKS — Wreck" },
  { value: "OBSTRN", label: "OBSTRN — Obstruction" },
  { value: "UWTROC", label: "UWTROC — Underwater Rock" },
  { value: "DEPARE", label: "DEPARE — Depth Area" },
  { value: "SOUNDG", label: "SOUNDG — Soundings" },
  { value: "COALNE", label: "COALNE — Coastline" },
  { value: "LNDARE", label: "LNDARE — Land Area" },
];

interface EditableFeature {
  id: string;
  object_class: S57ObjectClass;
  longitude: string;
  latitude: string;
  sounding: string; // VALSOU attribute
  attrs: string; // extra attributes as "LABEL=VALUE;LABEL=VALUE"
}

export function S57ExportDialog({ open, onClose }: Props) {
  const [features, setFeatures] = useState<EditableFeature[]>([
    {
      id: "feat_1",
      object_class: "WRECKS",
      longitude: "130.8456",
      latitude: "-12.3456",
      sounding: "25.0",
      attrs: "QUASOU=6;WATLEV=3",
    },
  ]);
  const [exportPath, setExportPath] = useState("/tmp/metardu_export.000");
  const [exporting, setExporting] = useState(false);
  const [result, setResult] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  if (!open) return null;

  function addFeature() {
    setFeatures((prev) => [
      ...prev,
      {
        id: `feat_${Date.now()}`,
        object_class: "OBSTRN",
        longitude: "",
        latitude: "",
        sounding: "",
        attrs: "",
      },
    ]);
  }

  function removeFeature(id: string) {
    setFeatures((prev) => prev.filter((f) => f.id !== id));
  }

  function updateFeature(id: string, field: keyof EditableFeature, value: string) {
    setFeatures((prev) =>
      prev.map((f) => (f.id === id ? { ...f, [field]: value } : f)),
    );
  }

  async function handleExport() {
    setExporting(true);
    setError(null);
    setResult(null);

    try {
      const s57Features: S57Feature[] = features
        .map((f) => {
          const lon = parseFloat(f.longitude);
          const lat = parseFloat(f.latitude);
          if (isNaN(lon) || isNaN(lat)) return null;

          const geometry: S57Geometry = {
            type: "point",
            longitude: lon,
            latitude: lat,
          };

          const attributes: S57Attribute[] = [];
          if (f.sounding) {
            attributes.push({ label: "VALSOU", value: f.sounding });
          }
          if (f.attrs) {
            f.attrs.split(";").forEach((pair) => {
              const [label, value] = pair.split("=").map((s) => s.trim());
              if (label && value) {
                attributes.push({ label, value });
              }
            });
          }

          return {
            object_class: f.object_class,
            geometry,
            attributes,
          } as S57Feature;
        })
        .filter((f): f is S57Feature => f !== null);

      if (s57Features.length === 0) {
        setError("No valid features. Check longitude/latitude values.");
        setExporting(false);
        return;
      }

      const ok = await exportS57(s57Features, exportPath);
      if (ok) {
        setResult(`Exported ${s57Features.length} features to ${exportPath}`);
      } else {
        setError("Browser mode — S-57 export requires the native Tauri shell");
      }
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setExporting(false);
    }
  }

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
      onClick={onClose}
    >
      <div
        onClick={(e) => e.stopPropagation()}
        className="flex max-h-[85vh] w-full max-w-3xl flex-col rounded-lg border border-navy-border bg-navy-panel shadow-2xl"
      >
        {/* Header */}
        <div className="flex items-center justify-between border-b border-navy-border px-5 py-3">
          <h2 className="flex items-center gap-2 text-sm font-semibold text-white">
            <Anchor className="h-4 w-4" style={{ color: colors.marineTurquoise }} />
            S-57 ENC Export
          </h2>
          <button
            onClick={onClose}
            className="rounded p-1 text-steel-gray hover:bg-navy-elevated hover:text-white"
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        {/* Body */}
        <div className="flex-1 overflow-y-auto p-5">
          {/* Features list */}
          <div className="mb-4">
            <div className="mb-2 flex items-center justify-between">
              <span className="text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
                Features ({features.length})
              </span>
              <button
                onClick={addFeature}
                className="flex items-center gap-1 rounded-md border border-navy-border bg-navy-base px-2 py-1 text-[10px] text-steel-light hover:bg-navy-elevated"
              >
                <Plus className="h-3 w-3" />
                Add feature
              </button>
            </div>

            {/* Feature table */}
            <div className="overflow-x-auto rounded-md border border-navy-border">
              <table className="w-full text-left text-[10px]">
                <thead className="bg-navy-base text-steel-gray">
                  <tr>
                    <th className="px-2 py-1.5">Object Class</th>
                    <th className="px-2 py-1.5">Longitude</th>
                    <th className="px-2 py-1.5">Latitude</th>
                    <th className="px-2 py-1.5">Sounding (m)</th>
                    <th className="px-2 py-1.5">Extra Attrs</th>
                    <th className="px-2 py-1.5"></th>
                  </tr>
                </thead>
                <tbody>
                  {features.map((f) => (
                    <tr key={f.id} className="border-t border-navy-border">
                      <td className="px-2 py-1.5">
                        <select
                          value={f.object_class}
                          onChange={(e) =>
                            updateFeature(f.id, "object_class", e.target.value)
                          }
                          className="w-full rounded border border-navy-border bg-navy-base px-1 py-1 text-[10px] text-white focus:border-industrial-orange focus:outline-none"
                        >
                          {OBJECT_CLASSES.map((oc) => (
                            <option key={oc.value} value={oc.value}>
                              {oc.label}
                            </option>
                          ))}
                        </select>
                      </td>
                      <td className="px-2 py-1.5">
                        <input
                          type="text"
                          value={f.longitude}
                          onChange={(e) => updateFeature(f.id, "longitude", e.target.value)}
                          className="w-24 rounded border border-navy-border bg-navy-base px-1 py-1 font-mono text-[10px] text-white focus:border-industrial-orange focus:outline-none"
                          placeholder="130.8456"
                        />
                      </td>
                      <td className="px-2 py-1.5">
                        <input
                          type="text"
                          value={f.latitude}
                          onChange={(e) => updateFeature(f.id, "latitude", e.target.value)}
                          className="w-24 rounded border border-navy-border bg-navy-base px-1 py-1 font-mono text-[10px] text-white focus:border-industrial-orange focus:outline-none"
                          placeholder="-12.3456"
                        />
                      </td>
                      <td className="px-2 py-1.5">
                        <input
                          type="text"
                          value={f.sounding}
                          onChange={(e) => updateFeature(f.id, "sounding", e.target.value)}
                          className="w-16 rounded border border-navy-border bg-navy-base px-1 py-1 font-mono text-[10px] text-white focus:border-industrial-orange focus:outline-none"
                          placeholder="25.0"
                        />
                      </td>
                      <td className="px-2 py-1.5">
                        <input
                          type="text"
                          value={f.attrs}
                          onChange={(e) => updateFeature(f.id, "attrs", e.target.value)}
                          className="w-40 rounded border border-navy-border bg-navy-base px-1 py-1 font-mono text-[10px] text-white focus:border-industrial-orange focus:outline-none"
                          placeholder="QUASOU=6;WATLEV=3"
                        />
                      </td>
                      <td className="px-2 py-1.5">
                        <button
                          onClick={() => removeFeature(f.id)}
                          className="text-steel-gray hover:text-fail"
                        >
                          <Trash2 className="h-3 w-3" />
                        </button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>

          {/* Export path */}
          <section className="mb-4">
            <label className="mb-1.5 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
              Export file path (.000)
            </label>
            <input
              type="text"
              value={exportPath}
              onChange={(e) => setExportPath(e.target.value)}
              className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 font-mono text-sm text-white focus:border-industrial-orange focus:outline-none"
              placeholder="/path/to/survey.000"
            />
            <p className="mt-1 text-[10px] text-steel-gray">
              The .000 file can be ingested by CARIS S-57 Composer or any S-57 reader.
            </p>
          </section>

          {/* Error / Result */}
          {error && (
            <div
              className="mb-4 rounded-md border p-3 text-xs"
              style={{ borderColor: `${colors.fail}40`, background: `${colors.fail}10`, color: colors.fail }}
            >
              {error}
            </div>
          )}
          {result && (
            <div
              className="mb-4 rounded-md border p-3 text-xs"
              style={{ borderColor: `${colors.pass}40`, background: `${colors.pass}10`, color: colors.pass }}
            >
              ✓ {result}
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between border-t border-navy-border px-5 py-3">
          <div className="text-[10px] text-steel-gray">
            IHO S-57 Edition 3.1 · ISO 8211 binary · Phase 2 simplified writer
          </div>
          <button
            onClick={handleExport}
            disabled={exporting || features.length === 0}
            className="flex items-center gap-1.5 rounded-md px-4 py-1.5 text-xs font-medium transition-colors disabled:opacity-40"
            style={{
              background: exporting ? colors.steelGray : colors.marineTurquoise,
              color: colors.navyBase,
            }}
          >
            <Download className="h-3 w-3" />
            {exporting ? "Exporting…" : "Export S-57"}
          </button>
        </div>
      </div>
    </div>
  );
}
