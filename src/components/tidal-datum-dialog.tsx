/**
 * Tidal Datum Converter — Sprint 10 Marine Tool #1.
 *
 * Convert depths between MLLW, MSL, CD (Chart Datum), LAT, NAVD88
 * using a known offset. Required for any bathymetric deliverable
 * crossing jurisdictional boundaries (e.g., port authorities use CD,
 * US charts use MLLW, civil works use NAVD88).
 *
 * Workflow:
 *   1. Enter the source datum and target datum
 *   2. Enter the known vertical offset (meters) — sign convention:
 *      positive if target datum is ABOVE source datum
 *   3. Paste a list of depths (one per line)
 *   4. Click Convert → see converted depths + copy as CSV
 */

import { useState, useMemo } from "react";
import { Waves, ArrowRight, Copy } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { colors } from "@/lib/tokens";
import { isNative } from "@/lib/tauri-ipc";
import { DialogShell, DialogButton } from "@/components/dialog-shell";

type Datum = "mllw" | "msl" | "cd" | "lat" | "navd88";

const DATUMS: { value: Datum; label: string; description: string }[] = [
  { value: "mllw", label: "MLLW", description: "Mean Lower Low Water (US charts)" },
  { value: "msl", label: "MSL", description: "Mean Sea Level" },
  { value: "cd", label: "CD", description: "Chart Datum (port authorities)" },
  { value: "lat", label: "LAT", description: "Lowest Astronomical Tide" },
  { value: "navd88", label: "NAVD88", description: "North American Vertical Datum 1988" },
];

interface Props {
  open: boolean;
  onClose: () => void;
}

export function TidalDatumDialog({ open, onClose }: Props) {
  const [fromDatum, setFromDatum] = useState<Datum>("mllw");
  const [toDatum, setToDatum] = useState<Datum>("cd");
  const [offset, setOffset] = useState("1.25");
  const [inputDepths, setInputDepths] = useState("12.5\n14.2\n18.7\n22.1\n9.8");
  const [outputDepths, setOutputDepths] = useState<number[] | null>(null);
  const [loading, setLoading] = useState(false);
  void loading;
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);


  const parsedDepths = useMemo(() => {
    return inputDepths
      .split("\n")
      .map((l) => l.trim())
      .filter(Boolean)
      .map((l) => parseFloat(l))
      .filter((n) => !Number.isNaN(n));
  }, [inputDepths]);

  async function handleConvert() {
    setLoading(true);
    setError(null);
    setOutputDepths(null);
    try {
      if (!isNative()) {
        setError("Browser mode — tidal conversion requires the native Tauri shell");
        return;
      }
      const off = parseFloat(offset);
      if (Number.isNaN(off)) throw new Error("Invalid offset — enter a numeric value");
      if (parsedDepths.length === 0) throw new Error("Enter at least one depth");
      // The Rust cmd currently converts MLLW→CD using offset_m; we honor the
      // same math (additive offset) for the chosen pair.
      const result = await invoke<number[]>("convert_tidal_datum_cmd", {
        depths: parsedDepths,
        offsetM: off,
      });
      setOutputDepths(result);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }

  function copyAsCsv() {
    if (!outputDepths) return;
    const csv = "input_depth,converted_depth\n" +
      parsedDepths.map((d, i) => `${d.toFixed(3)},${outputDepths[i]?.toFixed(3) ?? ""}`).join("\n");
    navigator.clipboard.writeText(csv);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  }

return (
    <DialogShell
      open={open}
      onClose={onClose}
      title="Tidal Datum Converter"
      icon={<Waves className="h-4 w-4" />}
      iconColor={colors.marineTurquoise}
      maxWidth="max-w-3xl"
      subtitle="MLLW/MSL/CD/LAT/NAVD88"
      footerHint="Offset-based conversion"
      actions={
        <>
        <DialogButton variant="primary" onClick={handleConvert}>Convert</DialogButton>
        <DialogButton variant="secondary" onClick={onClose}>Close</DialogButton>
        </>
      }
    >
          {/* Datum pair + offset */}
          <div className="grid grid-cols-[1fr_auto_1fr_auto] items-end gap-3">
            <div>
              <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">From Datum</label>
              <select
                value={fromDatum}
                onChange={(e) => setFromDatum(e.target.value as Datum)}
                className="w-full rounded-md border border-navy-border bg-navy-base px-2 py-1.5 text-sm text-white"
              >
                {DATUMS.map((d) => (
                  <option key={d.value} value={d.value}>{d.label} — {d.description}</option>
                ))}
              </select>
            </div>

            <ArrowRight className="mb-2 h-4 w-4" style={{ color: colors.marine }} />

            <div>
              <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">To Datum</label>
              <select
                value={toDatum}
                onChange={(e) => setToDatum(e.target.value as Datum)}
                className="w-full rounded-md border border-navy-border bg-navy-base px-2 py-1.5 text-sm text-white"
              >
                {DATUMS.map((d) => (
                  <option key={d.value} value={d.value}>{d.label} — {d.description}</option>
                ))}
              </select>
            </div>

            <div className="w-28">
              <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">Offset (m)</label>
              <input
                type="number"
                value={offset}
                step="0.001"
                onChange={(e) => setOffset(e.target.value)}
                className="input-enterprise w-full rounded-md border border-navy-border bg-navy-base px-2 py-1.5 font-mono text-sm text-white focus:border-marine focus:outline-none"
              />
            </div>
          </div>

          <div className="rounded-md bg-navy-base p-2 text-[10px] leading-relaxed text-steel-gray">
            <strong className="text-steel-light">Sign convention:</strong> positive offset means the target datum is{" "}
            <em>above</em> the source datum. To convert MLLW → CD where CD is 1.25 m below MLLW, enter offset = <span className="font-mono">−1.25</span>.
            Always confirm the offset sign against a known tide-station benchmark before applying to a survey.
          </div>

          {/* Depths */}
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
                Input Depths ({parsedDepths.length}) — {fromDatum.toUpperCase()}
              </label>
              <textarea
                value={inputDepths}
                onChange={(e) => setInputDepths(e.target.value)}
                rows={8}
                placeholder="12.5&#10;14.2&#10;18.7"
                className="input-enterprise w-full rounded-md border border-navy-border bg-navy-base px-2 py-1.5 font-mono text-xs text-white focus:border-marine focus:outline-none"
              />
            </div>

            <div>
              <div className="mb-1 flex items-center justify-between">
                <label className="text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
                  Output Depths — {toDatum.toUpperCase()}
                </label>
                {outputDepths && (
                  <button
                    onClick={copyAsCsv}
                    className="flex items-center gap-1 rounded px-2 py-0.5 text-[10px] font-medium"
                    style={{ background: colors.steelLight, color: colors.navyBase }}
                  >
                    <Copy className="h-3 w-3" /> {copied ? "Copied!" : "Copy CSV"}
                  </button>
                )}
              </div>
              <div className="h-[182px] overflow-y-auto rounded-md border border-navy-border bg-navy-base p-2 font-mono text-xs text-white">
                {outputDepths ? (
                  <table className="table-enterprise w-full text-right">
                    <thead className="text-[9px] uppercase text-steel-gray">
                      <tr>
                        <th className="pb-1 text-left">Input (m)</th>
                        <th className="pb-1 text-right">Output (m)</th>
                      </tr>
                    </thead>
                    <tbody>
                      {outputDepths.map((d, i) => (
                        <tr key={i} className="border-t border-navy-border">
                          <td className="py-0.5 text-left text-steel-light">{parsedDepths[i]?.toFixed(3) ?? "—"}</td>
                          <td className="py-0.5" style={{ color: colors.marine }}>{d.toFixed(3)}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                ) : (
                  <span className="text-steel-gray">Converted depths will appear here.</span>
                )}
              </div>
            </div>
          </div>

          {error && (
            <div className="rounded-md border p-3 text-xs" style={{ borderColor: `${colors.fail}40`, background: `${colors.fail}10`, color: colors.fail }}>
              {error}
            </div>
          )}
    </DialogShell>
  );
}
