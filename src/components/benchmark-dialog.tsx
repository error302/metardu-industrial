/**
 * Performance Benchmark Dialog — Sprint 7.
 *
 * Run the benchmark suite and display results. Used for:
 *   - Marketing claims ("handles 1M points in 0.8s on Toughbook")
 *   - Hardware spec verification ("does my machine meet recommended?")
 *   - Regression detection (compare across releases)
 */

import { useState } from "react";
import {
  Loader2, Gauge, CheckCircle2, AlertTriangle, Cpu, Clock, Activity,
} from "lucide-react";
import { colors } from "@/lib/tokens";
import { DialogShell, DialogButton } from "@/components/dialog-shell";
import {
  runBenchmarks,
  type BenchmarkSuiteResult,
  type BenchmarkResult,
} from "@/lib/tauri-ipc";

interface Props {
  open: boolean;
  onClose: () => void;
}

export function BenchmarkDialog({ open, onClose }: Props) {
  const [running, setRunning] = useState(false);
  const [result, setResult] = useState<BenchmarkSuiteResult | null>(null);
  const [error, setError] = useState<string | null>(null);


  async function handleRun() {
    setRunning(true);
    setError(null);
    setResult(null);
    try {
      const r = await runBenchmarks(5);
      if (r) {
        setResult(r);
      } else {
        setError("Browser mode — benchmarks require the native Tauri shell");
      }
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setRunning(false);
    }
  }

return (
    <DialogShell
      open={open}
      onClose={onClose}
      title="Performance Benchmark"
      icon={<Gauge className="h-4 w-4" />}
      iconColor={colors.steelLight}
      maxWidth="max-w-2xl"
      subtitle="8 benchmarks + p95 timing"
      footerHint="Throughput measurement"
      actions={
        <DialogButton variant="secondary" onClick={onClose}>Close</DialogButton>
      }
    >
          {error && (
            <div className="rounded-md border p-3 text-xs"
              style={{ borderColor: `${colors.fail}40`, background: `${colors.fail}10`, color: colors.fail }}>
              {error}
            </div>
          )}

          {/* Run button + system info */}
          <div className="flex items-center gap-3">
            <button
              onClick={handleRun}
              disabled={running}
              className="flex items-center gap-2 rounded-md px-5 py-2 text-sm font-bold transition-colors disabled:opacity-40"
              style={{ background: colors.industrialOrange, color: colors.navyBase }}
            >
              {running ? <Loader2 className="h-4 w-4 animate-spin" /> : <Gauge className="h-4 w-4" />}
              {running ? "Running…" : "Run Benchmarks (5 iterations)"}
            </button>
            {result && (
              <div className="flex items-center gap-3 text-xs text-steel-light">
                <span className="flex items-center gap-1">
                  <Cpu className="h-3 w-3" />
                  {result.system_info.os} {result.system_info.arch} · {result.system_info.cpu_count} CPUs
                </span>
                <span className="flex items-center gap-1">
                  <Clock className="h-3 w-3" />
                  {result.total_duration_secs.toFixed(1)}s total
                </span>
                {result.overall_pass ? (
                  <span className="flex items-center gap-1" style={{ color: colors.pass }}>
                    <CheckCircle2 className="h-3 w-3" /> All passed
                  </span>
                ) : (
                  <span className="flex items-center gap-1" style={{ color: colors.fail }}>
                    <AlertTriangle className="h-3 w-3" /> Some failed
                  </span>
                )}
              </div>
            )}
          </div>

          {/* Results table */}
          {result && (
            <div className="rounded-md border border-navy-border overflow-hidden">
              <table className="table-enterprise w-full text-left text-xs">
                <thead className="bg-navy-panel text-steel-gray">
                  <tr>
                    <th className="px-3 py-2">Benchmark</th>
                    <th className="px-3 py-2 text-right">Min (ms)</th>
                    <th className="px-3 py-2 text-right">Mean (ms)</th>
                    <th className="px-3 py-2 text-right">P95 (ms)</th>
                    <th className="px-3 py-2 text-right">Throughput</th>
                    <th className="px-3 py-2 text-center">Status</th>
                  </tr>
                </thead>
                <tbody>
                  {result.results.map((r: BenchmarkResult) => (
                    <tr key={r.name} className="border-t border-navy-border">
                      <td className="px-3 py-2">
                        <div className="font-mono text-white">{r.name}</div>
                        <div className="text-[10px] text-steel-gray">{r.description}</div>
                      </td>
                      <td className="px-3 py-2 text-right font-mono text-steel-light">{r.min_ms.toFixed(1)}</td>
                      <td className="px-3 py-2 text-right font-mono text-white">{r.mean_ms.toFixed(1)}</td>
                      <td className="px-3 py-2 text-right font-mono text-steel-light">{r.p95_ms.toFixed(1)}</td>
                      <td className="px-3 py-2 text-right font-mono text-steel-light">
                        {r.throughput
                          ? `${formatThroughput(r.throughput.value)} ${r.throughput.unit}`
                          : "—"}
                      </td>
                      <td className="px-3 py-2 text-center">
                        {r.passed ? (
                          <CheckCircle2 className="inline h-3.5 w-3.5" style={{ color: colors.pass }} />
                        ) : (
                          <AlertTriangle className="inline h-3.5 w-3.5" style={{ color: colors.fail }} />
                        )}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}

          {/* Notes */}
          {result && (
            <div className="input-enterprise rounded-md border border-navy-border bg-navy-base p-3 text-[10px] text-steel-gray">
              <div className="mb-1 flex items-center gap-1 font-semibold text-steel-light">
                <Activity className="h-3 w-3" /> Notes
              </div>
              <ul className="space-y-0.5 list-disc pl-4">
                {result.results.map((r) => (
                  <li key={r.name}>
                    <span className="font-mono text-steel-light">{r.name}</span>: {r.notes}
                  </li>
                ))}
              </ul>
            </div>
          )}

          {!result && !running && (
            <div className="input-enterprise rounded-md border border-navy-border bg-navy-base p-8 text-center text-xs text-steel-gray">
              Click "Run Benchmarks" to measure your hardware's performance.
              The suite takes ~30 seconds and covers point cloud loading, CSF classification,
              volume calc, dredge audit, highwall analysis, license verification, SHA-256, and JSON serialization.
            </div>
          )}
    </DialogShell>
  );
}

function formatThroughput(value: number): string {
  if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(2)}M`;
  if (value >= 1000) return `${(value / 1000).toFixed(1)}K`;
  return value.toFixed(0);
}
