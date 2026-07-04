import { useEscapeKey } from "@/lib/use-escape-key";
/**
 * CUBE Hypothesis Disambiguation — Sprint 3 Priority #6.
 *
 * Shows which CUBE grid cells have multiple depth hypotheses (ambiguous).
 * Surveyors click a cell to see alternative depth estimates and manually
 * select the correct one.
 *
 * Turns CUBE from a black box into an interactive QC tool.
 */

import { useState, useMemo } from "react";
import { X, Waves, AlertTriangle, Check, ChevronRight } from "lucide-react";
import { colors } from "@/lib/tokens";
import type { CubeSurfaceRpc } from "@/lib/tauri-ipc";

interface Props {
  open: boolean;
  onClose: () => void;
  surface: CubeSurfaceRpc | null;
}

export function CubeDisambiguationDialog({ open, onClose, surface }: Props) {
  const [selectedCell, setSelectedCell] = useState<number | null>(null);
  const [resolvedCells, setResolvedCells] = useState<Set<number>>(new Set());

  useEscapeKey(onClose, open);
  if (!open || !surface) return null;

  // Find ambiguous cells (hypothesis_count > 1)
  const ambiguousCells = useMemo(() => {
    const cells: number[] = [];
    for (let i = 0; i < surface.hypothesis_counts.length; i++) {
      if (surface.hypothesis_counts[i] > 1 && !Number.isNaN(surface.depths[i])) {
        cells.push(i);
      }
    }
    return cells;
  }, [surface]);

  const [cols, rows] = surface.dims;
  const unresolved = ambiguousCells.filter((i) => !resolvedCells.has(i));

  // SVG heatmap of hypothesis counts
  const cellSize = Math.min(400 / cols, 300 / rows);
  const gridW = cols * cellSize;
  const gridH = rows * cellSize;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
      onClick={onClose}
    >
      <div
        onClick={(e) => e.stopPropagation()}
        className="flex max-h-[88vh] w-full max-w-3xl flex-col rounded-lg border border-navy-border bg-navy-panel shadow-2xl"
      >
        {/* Header */}
        <div className="flex items-center justify-between border-b border-navy-border px-5 py-3">
          <h2 className="flex items-center gap-2 text-sm font-semibold text-white">
            <Waves className="h-4 w-4" style={{ color: colors.marineTurquoise }} />
            CUBE Hypothesis Disambiguation
          </h2>
          <button onClick={onClose} className="rounded p-1 text-steel-gray hover:bg-navy-elevated hover:text-white">
            <X className="h-4 w-4" />
          </button>
        </div>

        {/* Body */}
        <div className="flex-1 overflow-y-auto p-5">
          {/* Summary */}
          <div className="mb-4 grid grid-cols-4 gap-2">
            <div className="rounded-md border p-2.5" style={{ borderColor: `${colors.investigate}40`, background: `${colors.investigate}10` }}>
              <div className="text-[9px] uppercase tracking-wider" style={{ color: colors.investigate }}>Ambiguous</div>
              <div className="mt-0.5 font-mono text-sm font-bold text-white">{unresolved.length}</div>
            </div>
            <div className="rounded-md border p-2.5" style={{ borderColor: `${colors.pass}40`, background: `${colors.pass}10` }}>
              <div className="text-[9px] uppercase tracking-wider" style={{ color: colors.pass }}>Resolved</div>
              <div className="mt-0.5 font-mono text-sm font-bold text-white">{resolvedCells.size}</div>
            </div>
            <div className="rounded-md border p-2.5" style={{ borderColor: `${colors.steelLight}40`, background: `${colors.steelLight}10` }}>
              <div className="text-[9px] uppercase tracking-wider" style={{ color: colors.steelLight }}>Valid cells</div>
              <div className="mt-0.5 font-mono text-sm font-bold text-white">{surface.valid_cells}</div>
            </div>
            <div className="rounded-md border p-2.5" style={{ borderColor: `${colors.marineTurquoise}40`, background: `${colors.marineTurquoise}10` }}>
              <div className="text-[9px] uppercase tracking-wider" style={{ color: colors.marineTurquoise }}>Total soundings</div>
              <div className="mt-0.5 font-mono text-sm font-bold text-white">{surface.total_soundings.toLocaleString()}</div>
            </div>
          </div>

          {unresolved.length === 0 && resolvedCells.size > 0 && (
            <div className="mb-4 flex items-center gap-2 rounded-md border p-3" style={{ borderColor: `${colors.pass}40`, background: `${colors.pass}10` }}>
              <Check className="h-4 w-4" style={{ color: colors.pass }} />
              <span className="text-sm font-semibold" style={{ color: colors.pass }}>All ambiguous cells resolved</span>
            </div>
          )}

          {/* Heatmap */}
          <div className="flex gap-4">
            <div className="flex-1">
              <div className="mb-2 text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
                Hypothesis Count Heatmap (click to inspect)
              </div>
              <div className="rounded-md border border-navy-border bg-navy-base p-3 flex justify-center">
                <svg width={gridW + 40} height={gridH + 40} style={{ maxWidth: "100%" }}>
                  {/* Grid cells */}
                  {Array.from({ length: rows }, (_, r) =>
                    Array.from({ length: cols }, (_, c) => {
                      const idx = r * cols + c;
                      const hc = surface.hypothesis_counts[idx];
                      const depth = surface.depths[idx];
                      const isResolved = resolvedCells.has(idx);
                      const isSelected = selectedCell === idx;

                      if (Number.isNaN(depth)) return null;

                      let fillColor: string = colors.navyBorder;
                      if (hc > 1 && !isResolved) fillColor = colors.investigate;
                      else if (isResolved) fillColor = colors.pass;
                      else fillColor = colors.marineTurquoise;

                      return (
                        <rect
                          key={idx}
                          x={20 + c * cellSize}
                          y={20 + r * cellSize}
                          width={cellSize}
                          height={cellSize}
                          fill={fillColor}
                          opacity={isSelected ? 1 : 0.7}
                          stroke={isSelected ? colors.industrialOrange : "none"}
                          strokeWidth={isSelected ? 2 : 0}
                          onClick={() => hc > 1 && setSelectedCell(idx)}
                          style={{ cursor: hc > 1 && !isResolved ? "pointer" : "default" }}
                        />
                      );
                    })
                  )}

                  {/* Legend */}
                  <rect x={20} y={gridH + 28} width={12} height={12} fill={colors.marineTurquoise} opacity={0.7} />
                  <text x={36} y={38 + gridH} fill={colors.steelGray} fontSize="9" fontFamily="JetBrains Mono">1 hypothesis</text>
                  <rect x={120} y={gridH + 28} width={12} height={12} fill={colors.investigate} opacity={0.7} />
                  <text x={136} y={38 + gridH} fill={colors.steelGray} fontSize="9" fontFamily="JetBrains Mono">ambiguous</text>
                  <rect x={220} y={gridH + 28} width={12} height={12} fill={colors.pass} opacity={0.7} />
                  <text x={236} y={38 + gridH} fill={colors.steelGray} fontSize="9" fontFamily="JetBrains Mono">resolved</text>
                </svg>
              </div>
            </div>

            {/* Cell inspector */}
            {selectedCell !== null && (
              <div className="w-64 rounded-md border border-navy-border bg-navy-base p-3">
                <div className="mb-2 flex items-center gap-1.5">
                  <AlertTriangle className="h-3.5 w-3.5" style={{ color: colors.investigate }} />
                  <span className="text-[10px] font-semibold uppercase tracking-wider" style={{ color: colors.investigate }}>
                    Cell #{selectedCell}
                  </span>
                </div>

                <div className="space-y-2 text-[11px]">
                  <div className="flex justify-between">
                    <span className="text-steel-gray">Depth (selected):</span>
                    <span className="font-mono text-white">{surface.depths[selectedCell].toFixed(2)} m</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-steel-gray">Uncertainty:</span>
                    <span className="font-mono text-white">±{surface.uncertainties[selectedCell].toFixed(3)} m</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-steel-gray">Soundings:</span>
                    <span className="font-mono text-white">{surface.sounding_counts[selectedCell]}</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-steel-gray">Hypotheses:</span>
                    <span className="font-mono" style={{ color: colors.investigate }}>{surface.hypothesis_counts[selectedCell]}</span>
                  </div>
                </div>

                <div className="mt-3 border-t border-navy-border pt-3">
                  <p className="mb-2 text-[10px] text-steel-gray">
                    Multiple hypotheses detected — possible artifact (fish school, weed, wreck).
                    Accept the current depth or flag for manual review.
                  </p>

                  {!resolvedCells.has(selectedCell) ? (
                    <div className="flex flex-col gap-1.5">
                      <button
                        onClick={() => {
                          setResolvedCells((prev) => new Set(prev).add(selectedCell));
                          setSelectedCell(null);
                        }}
                        className="flex items-center justify-center gap-1.5 rounded-md py-1.5 text-[11px] font-medium"
                        style={{ background: colors.pass, color: colors.navyBase }}
                      >
                        <Check className="h-3 w-3" /> Accept depth
                      </button>
                      <button
                        onClick={() => setSelectedCell(null)}
                        className="flex items-center justify-center gap-1.5 rounded-md py-1.5 text-[11px] text-steel-light hover:text-white"
                      >
                        <ChevronRight className="h-3 w-3" /> Skip
                      </button>
                    </div>
                  ) : (
                    <div className="text-[10px]" style={{ color: colors.pass }}>✓ Resolved</div>
                  )}
                </div>
              </div>
            )}
          </div>
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between border-t border-navy-border px-5 py-3">
          <div className="text-[10px] text-steel-gray">
            {unresolved.length} unresolved · {resolvedCells.size} accepted
          </div>
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
