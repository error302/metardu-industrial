/**
 * Mine Grid Transform Tool — Sprint 10 Mining Field Tool #2.
 *
 * Bidirectional conversion between a mine's local grid (e.g., "NEWMONT-A")
 * and the parent projected CRS (e.g., EPSG:28355 — MGA94 Zone 55). Includes
 * rotation + scale, which is required for any mine with a grid aligned to
 * a pit orientation rather than true north.
 *
 * Use cases:
 *   - Convert design coords from mine grid to total-station CRS for setout
 *   - Convert field-surveyed CRS coords back to mine grid for mine-plan update
 *   - Validate mine-grid origin / rotation against known points
 */

import { useState } from "react";
import { X, Grid3x3, ArrowRightLeft, Loader2 } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { colors } from "@/lib/tokens";
import { isNative } from "@/lib/tauri-ipc";
import { useEscapeKey } from "@/lib/use-escape-key";
import { ValidatedNumberInput } from "@/components/validated-number-input";

interface MineGrid {
  name: string;
  origin_easting: number;
  origin_northing: number;
  rotation_deg: number;
  scale_factor: number;
  parent_crs: string;
}

interface Props {
  open: boolean;
  onClose: () => void;
}

type Direction = "grid_to_crs" | "crs_to_grid";

export function MineGridDialog({ open, onClose }: Props) {
  const [grid, setGrid] = useState<MineGrid>({
    name: "MINE-A",
    origin_easting: 283550.000,
    origin_northing: 6245000.000,
    rotation_deg: 12.5,
    scale_factor: 1.0,
    parent_crs: "EPSG:28355",
  });
  const [dir, setDir] = useState<Direction>("grid_to_crs");
  const [inputE, setInputE] = useState("1000.000");
  const [inputN, setInputN] = useState("500.000");
  const [output, setOutput] = useState<{ e: number; n: number } | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEscapeKey(onClose, open);
  if (!open) return null;

  async function handleConvert() {
    setLoading(true);
    setError(null);
    setOutput(null);
    try {
      if (!isNative()) {
        setError("Browser mode — mine grid transform requires the native Tauri shell");
        return;
      }
      const e = parseFloat(inputE);
      const n = parseFloat(inputN);
      if (Number.isNaN(e) || Number.isNaN(n)) {
        throw new Error("Invalid input coordinate — enter numeric values");
      }
      let result: [number, number];
      if (dir === "grid_to_crs") {
        result = await invoke("mine_grid_to_crs_cmd", {
          grid,
          gridEasting: e,
          gridNorthing: n,
        });
      } else {
        result = await invoke("crs_to_mine_grid_cmd", {
          grid,
          crsEasting: e,
          crsNorthing: n,
        });
      }
      setOutput({ e: result[0], n: result[1] });
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }

  const inputLabel = dir === "grid_to_crs" ? "Mine Grid Coord" : `Parent CRS (${grid.parent_crs})`;
  const outputLabel = dir === "grid_to_crs" ? `Parent CRS (${grid.parent_crs})` : "Mine Grid Coord";

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
      onClick={onClose}
    >
      <div
        onClick={(e) => e.stopPropagation()}
        className="flex max-h-[88vh] w-full max-w-2xl flex-col rounded-lg border border-navy-border bg-navy-panel shadow-2xl"
      >
        <div className="flex items-center justify-between border-b border-navy-border px-5 py-3">
          <h2 className="flex items-center gap-2 text-sm font-semibold text-white">
            <Grid3x3 className="h-4 w-4" style={{ color: colors.mining }} />
            Mine Grid Transform
          </h2>
          <button onClick={onClose} className="rounded p-1 text-steel-gray hover:bg-navy-elevated hover:text-white">
            <X className="h-4 w-4" />
          </button>
        </div>

        <div className="flex-1 overflow-y-auto p-5 space-y-4">
          {/* Grid Definition */}
          <section>
            <h3 className="mb-2 text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
              Mine Grid Definition
            </h3>
            <div className="grid grid-cols-2 gap-2 rounded-md border border-navy-border bg-navy-base p-3">
              <Field label="Grid Name" value={grid.name} onChange={(v) => setGrid({ ...grid, name: v })} />
              <Field label="Parent CRS" value={grid.parent_crs} onChange={(v) => setGrid({ ...grid, parent_crs: v })} />
              <Field
                label="Origin Easting (m)"
                value={String(grid.origin_easting)}
                onChange={(v) => setGrid({ ...grid, origin_easting: parseFloat(v) || 0 })}
                mono
              />
              <Field
                label="Origin Northing (m)"
                value={String(grid.origin_northing)}
                onChange={(v) => setGrid({ ...grid, origin_northing: parseFloat(v) || 0 })}
                mono
              />
              <Field
                label="Rotation (°, clockwise from grid N to true N)"
                value={String(grid.rotation_deg)}
                onChange={(v) => setGrid({ ...grid, rotation_deg: parseFloat(v) || 0 })}
                mono
              />
              <Field
                label="Scale Factor"
                value={String(grid.scale_factor)}
                onChange={(v) => setGrid({ ...grid, scale_factor: parseFloat(v) || 1 })}
                mono
              />
            </div>
          </section>

          {/* Direction Toggle */}
          <section>
            <div className="flex items-center gap-1 rounded-md border border-navy-border bg-navy-base p-1">
              <button
                onClick={() => setDir("grid_to_crs")}
                className={`flex-1 rounded px-3 py-1.5 text-xs font-medium ${dir === "grid_to_crs" ? "text-navy-base" : "text-steel-gray"}`}
                style={{ background: dir === "grid_to_crs" ? colors.mining : "transparent" }}
              >
                Grid → CRS
              </button>
              <button
                onClick={() => setDir("crs_to_grid")}
                className={`flex-1 rounded px-3 py-1.5 text-xs font-medium ${dir === "crs_to_grid" ? "text-navy-base" : "text-steel-gray"}`}
                style={{ background: dir === "crs_to_grid" ? colors.mining : "transparent" }}
              >
                CRS → Grid
              </button>
            </div>
          </section>

          {/* Conversion */}
          <section className="grid grid-cols-[1fr_auto_1fr] items-end gap-3">
            <div>
              <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">{inputLabel}</label>
              <div className="space-y-1.5">
                <ValidatedNumberInput
                  value={inputE}
                  onChange={setInputE}
                  validationType="custom"
                  step={0.001}
                  min={-1000000}
                  max={1000000}
                  placeholder="Easting"
                />
                <ValidatedNumberInput
                  value={inputN}
                  onChange={setInputN}
                  validationType="custom"
                  step={0.001}
                  min={-1000000}
                  max={1000000}
                  placeholder="Northing"
                />
              </div>
            </div>

            <button
              onClick={handleConvert}
              disabled={loading}
              className="mb-1 flex flex-col items-center gap-0.5 rounded-md px-3 py-2 text-xs font-medium disabled:opacity-40"
              style={{ background: colors.mining, color: colors.navyBase }}
              title="Convert"
            >
              {loading ? <Loader2 className="h-4 w-4 animate-spin" /> : <ArrowRightLeft className="h-4 w-4" />}
              <span className="text-[9px]">Convert</span>
            </button>

            <div>
              <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">{outputLabel}</label>
              <div className="space-y-1.5">
                <div className="w-full rounded-md border border-navy-border bg-navy-base px-2 py-1.5 font-mono text-sm text-white">
                  E: {output ? output.e.toFixed(3) : "—"}
                </div>
                <div className="w-full rounded-md border border-navy-border bg-navy-base px-2 py-1.5 font-mono text-sm text-white">
                  N: {output ? output.n.toFixed(3) : "—"}
                </div>
              </div>
            </div>
          </section>

          {error && (
            <div className="rounded-md border p-3 text-xs" style={{ borderColor: `${colors.fail}40`, background: `${colors.fail}10`, color: colors.fail }}>
              {error}
            </div>
          )}

          {/* Helper text */}
          <p className="rounded-md bg-navy-base p-2 text-[10px] leading-relaxed text-steel-gray">
            <strong className="text-steel-light">Notes:</strong> Rotation is measured clockwise from grid north to true north.
            A mine aligned to a pit strike of 12.5° uses <span className="font-mono">rotation_deg = 12.5</span>. The
            transform applies: <span className="font-mono">CRS = origin + scale × rotate(grid)</span>. Always validate
            against two known points before relying on the transform.
          </p>
        </div>

        <div className="flex items-center justify-between border-t border-navy-border px-5 py-3">
          <div className="text-[10px] text-steel-gray">Local grid ↔ parent CRS with rotation + scale</div>
          <button
            onClick={onClose}
            className="rounded-md px-4 py-1.5 text-xs font-medium"
            style={{ background: colors.steelGray, color: colors.navyBase }}
          >
            Close
          </button>
        </div>
      </div>
    </div>
  );
}

function Field({ label, value, onChange, mono }: { label: string; value: string; onChange: (v: string) => void; mono?: boolean }) {
  return (
    <div>
      <label className="mb-0.5 block text-[9px] uppercase tracking-wider text-steel-gray">{label}</label>
      <input
        type="text"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className={`w-full rounded border border-navy-border bg-navy-base px-2 py-1 text-xs text-white focus:border-mining focus:outline-none ${mono ? "font-mono" : ""}`}
      />
    </div>
  );
}
