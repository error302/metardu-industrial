/**
 * Machine Control Compiler — drop DXF → pick vendor → get .svd/.tp3/.top
 *
 * Single screen. No wizard. Drop file, pick vendor, compile, done.
 */

import { useState } from "react";
import {
  X, Loader2, Cpu, CheckCircle2, FileText, Download,
} from "lucide-react";
import { colors } from "@/lib/tokens";
import {
  compileMachineControl,
  type MachineControlVendor,
  type MachineControlResult,
} from "@/lib/tauri-ipc";
import { pickFile, pickSaveFile } from "@/lib/file-picker";

interface Props {
  open: boolean;
  onClose: () => void;
}

const VENDORS: { value: MachineControlVendor; label: string; ext: string }[] = [
  { value: "leica", label: "Leica iCON (.svd)", ext: "svd" },
  { value: "trimble", label: "Trimble GCS900 (.tp3)", ext: "tp3" },
  { value: "topcon", label: "Topcon 3D-MC (.top)", ext: "top" },
];

export function MachineControlTool({ open, onClose }: Props) {
  const [inputPath, setInputPath] = useState("");
  const [vendor, setVendor] = useState<MachineControlVendor>("leica");
  const [outputPath, setOutputPath] = useState("");
  const [running, setRunning] = useState(false);
  const [result, setResult] = useState<MachineControlResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  if (!open) return null;

  // Auto-set output path when input + vendor change
  function updateInput(path: string) {
    setInputPath(path);
    const base = path.replace(/\.(dxf|xml|landxml)$/i, "");
    const ext = VENDORS.find((v) => v.value === vendor)?.ext ?? "svd";
    setOutputPath(`${base}.${ext}`);
  }

  function updateVendor(v: MachineControlVendor) {
    setVendor(v);
    if (inputPath) {
      const base = inputPath.replace(/\.(dxf|xml|landxml)$/i, "");
      const ext = VENDORS.find((vendorOption) => vendorOption.value === v)?.ext ?? "svd";
      setOutputPath(`${base}.${ext}`);
    }
  }

  async function handleCompile() {
    if (!inputPath) return;
    setRunning(true);
    setError(null);
    setResult(null);
    try {
      const r = await compileMachineControl({
        input_path: inputPath,
        vendor,
        output_path: outputPath,
      });
      if (r) {
        setResult(r);
      } else {
        setError("Browser mode — machine control compilation requires the native Tauri shell");
      }
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setRunning(false);
    }
  }

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
            <Cpu className="h-4 w-4" style={{ color: colors.industrialOrange }} />
            Machine Control Compiler
          </h2>
          <button onClick={onClose} className="rounded p-1 text-steel-gray hover:bg-navy-elevated hover:text-white">
            <X className="h-4 w-4" />
          </button>
        </div>

        <div className="flex-1 overflow-y-auto p-5 space-y-4">
          {error && (
            <div className="rounded-md border p-3 text-xs"
              style={{ borderColor: `${colors.fail}40`, background: `${colors.fail}10`, color: colors.fail }}>
              {error}
            </div>
          )}

          {/* DXF input */}
          <div>
            <label className="mb-1 block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
              Input alignment file (DXF or LandXML)
            </label>
            <div className="flex items-center gap-2">
              <button
                onClick={async () => {
                  const p = await pickFile({ extensions: ["dxf", "xml", "landxml"], filterName: "DXF / LandXML", title: "Select alignment file" });
                  if (p) updateInput(p);
                }}
                className="flex items-center gap-1 rounded-md border border-navy-border bg-navy-base px-2.5 py-2 text-xs text-white hover:bg-navy-elevated"
              >
                <FileText className="h-3.5 w-3.5" /> Browse
              </button>
              <input
                type="text" value={inputPath} onChange={(e) => updateInput(e.target.value)}
                placeholder="Or type a path…"
                className="flex-1 rounded-md border border-navy-border bg-navy-base px-3 py-2 font-mono text-xs text-white focus:outline-none"
              />
            </div>
            <p className="mt-1 text-[10px] text-steel-gray">
              Supports: .dxf (POINT, LINE, LWPOLYLINE entities), .xml (LandXML — Phase 9)
            </p>
          </div>

          {/* Vendor selector */}
          <div>
            <label className="mb-1 block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
              Target machine control system
            </label>
            <div className="grid grid-cols-3 gap-2">
              {VENDORS.map((v) => (
                <button
                  key={v.value}
                  onClick={() => updateVendor(v.value)}
                  className="rounded-md border p-2.5 text-left text-xs transition-colors"
                  style={{
                    borderColor: vendor === v.value ? colors.industrialOrange : colors.navyBorder,
                    background: vendor === v.value ? `${colors.industrialOrange}10` : colors.navyBase,
                  }}
                >
                  <div className="font-semibold text-white">{v.label.split(" (")[0]}</div>
                  <div className="text-[10px] text-steel-gray">.{v.ext}</div>
                </button>
              ))}
            </div>
          </div>

          {/* Output path */}
          <div>
            <label className="mb-1 block text-[11px] font-semibold uppercase tracking-wider text-steel-light">
              Output file path
            </label>
            <div className="flex items-center gap-2">
              <button
                onClick={async () => {
                  const ext = VENDORS.find((v) => v.value === vendor)?.ext ?? "svd";
                  const p = await pickSaveFile({ extensions: [ext], filterName: VENDORS.find((v) => v.value === vendor)?.label ?? ext, title: "Save machine control file" });
                  if (p) setOutputPath(p);
                }}
                className="flex items-center gap-1 rounded-md border border-navy-border bg-navy-base px-2.5 py-2 text-xs text-white hover:bg-navy-elevated"
              >
                <Download className="h-3.5 w-3.5" /> Save As
              </button>
              <input
                type="text" value={outputPath} onChange={(e) => setOutputPath(e.target.value)}
                className="flex-1 rounded-md border border-navy-border bg-navy-base px-3 py-2 font-mono text-xs text-white focus:outline-none"
              />
            </div>
          </div>

          {/* Compile button */}
          <button
            onClick={handleCompile}
            disabled={!inputPath || !outputPath || running}
            className="flex items-center gap-2 rounded-md px-5 py-2.5 text-sm font-bold transition-colors disabled:opacity-40"
            style={{ background: colors.industrialOrange, color: colors.navyBase }}
          >
            {running ? <Loader2 className="h-4 w-4 animate-spin" /> : <Download className="h-4 w-4" />}
            {running ? "Compiling…" : `Compile to .${VENDORS.find((v) => v.value === vendor)?.ext}`}
          </button>

          {/* Result */}
          {result && (
            <div className="space-y-3">
              <div className="rounded-md border p-3"
                style={{ borderColor: `${colors.pass}40`, background: `${colors.pass}10` }}>
                <div className="flex items-center gap-2 mb-2">
                  <CheckCircle2 className="h-4 w-4" style={{ color: colors.pass }} />
                  <span className="text-sm font-bold text-white">Compilation Complete</span>
                </div>
                <div className="grid grid-cols-2 gap-2 text-xs">
                  <Stat label="Vendor" value={VENDORS.find((v) => v.value === result.vendor)?.label ?? result.vendor} />
                  <Stat label="Points" value={result.point_count.toLocaleString()} />
                  <Stat label="Lines/Polylines" value={result.line_count.toLocaleString()} />
                  <Stat label="File Size" value={`${(result.file_size_bytes / 1024).toFixed(1)} KB`} />
                  <Stat label="Output" value={result.output_path.split(/[\\/]/).pop() ?? result.output_path} />
                </div>
              </div>

              {result.warnings.length > 0 && (
                <div className="rounded-md border p-2 text-[10px]"
                  style={{ borderColor: "#F59E0B40", background: "#F59E0B10", color: "#F59E0B" }}>
                  {result.warnings.map((w, i) => <div key={i}>⚠ {w}</div>)}
                </div>
              )}

              <div className="text-[10px] text-steel-gray">
                Copy the output file to your machine's USB drive and load it into the
                machine control system. The file is ready for field use — no manual
                format conversion needed.
              </div>
            </div>
          )}
        </div>

        <div className="flex items-center justify-between border-t border-navy-border px-5 py-3 text-[10px] text-steel-gray">
          <span>Reads DXF/LandXML → compiles to vendor-specific binary. Eliminates format conversion headaches.</span>
          <button onClick={onClose}
            className="rounded-md px-3 py-1 text-xs font-medium"
            style={{ background: colors.pass, color: colors.navyBase }}>
            Close
          </button>
        </div>
      </div>
    </div>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <div className="text-[9px] uppercase tracking-wider text-steel-gray">{label}</div>
      <div className="font-mono text-white">{value}</div>
    </div>
  );
}
