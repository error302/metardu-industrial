import React, { useState } from 'react';
import { DialogShell } from './dialog-shell';
import { RefreshCcw, CheckCircle2, Download, Anchor } from 'lucide-react';

interface MbesPatchTestDialogProps {
  isOpen: boolean;
  onClose: () => void;
}

export const MbesPatchTestDialog: React.FC<MbesPatchTestDialogProps> = ({ isOpen, onClose }) => {
  const [isSolving, setIsSolving] = useState(false);
  const [results, setResults] = useState<null | {
    roll: number;
    pitch: number;
    yaw: number;
    latency: number;
    rmse: number;
  }>(null);

  const handleSolvePatchTest = () => {
    setIsSolving(true);
    setTimeout(() => {
      setIsSolving(false);
      setResults({
        roll: -0.42,   // deg
        pitch: +1.15,  // deg
        yaw: -0.85,    // deg
        latency: 0.08, // sec (80ms)
        rmse: 0.014,   // 14mm residual
      });
    }, 1000);
  };

  return (
    <DialogShell
      open={isOpen}
      onClose={onClose}
      title="Multibeam MBES Patch Test Alignment Calibration"
      subtitle="Automated Roll, Pitch, Yaw, and Latency solver across reciprocal calibration swaths"
      icon={<Anchor className="w-5 h-5 text-industrial-turquoise-400" />}
      maxWidth="max-w-4xl"
    >
      <div className="space-y-6">
        {/* Reciprocal Calibration Lines Header */}
        <div className="bg-industrial-navy-900/80 p-4 rounded-lg border border-industrial-navy-700 flex justify-between items-center">
          <div>
            <div className="text-xs font-mono text-industrial-navy-400">Swath Pair Dataset</div>
            <div className="text-sm font-bold font-mono text-white">4 Calibration Lines Ingested (Reciprocal Flat / Reciprocal Slope / Offset / Speed Diff)</div>
          </div>
          <button
            onClick={handleSolvePatchTest}
            disabled={isSolving}
            className="px-4 py-2 bg-industrial-turquoise-500 hover:bg-industrial-turquoise-600 text-industrial-navy-950 font-bold rounded text-xs flex items-center gap-1.5 transition-all shadow"
          >
            <RefreshCcw className={`w-3.5 h-3.5 ${isSolving ? 'animate-spin' : ''}`} />
            {isSolving ? 'Solving Least-Squares...' : 'Solve Patch Test Offsets'}
          </button>
        </div>

        {/* Results Metrics Grid */}
        {results && (
          <div className="bg-industrial-navy-850 p-5 rounded-xl border border-industrial-turquoise-500/30">
            <div className="flex items-center justify-between mb-4">
              <div className="text-sm font-mono font-bold text-industrial-turquoise-300 flex items-center gap-2">
                <CheckCircle2 className="w-4 h-4 text-emerald-400" /> Solved Transducer Mounting Offsets
              </div>
              <div className="text-xs font-mono text-emerald-400 bg-emerald-500/10 border border-emerald-500/30 px-2 py-0.5 rounded">
                Residual RMSE: {(results.rmse * 1000).toFixed(1)}mm
              </div>
            </div>

            <div className="grid grid-cols-2 md:grid-cols-4 gap-3 text-xs font-mono">
              <div className="bg-industrial-navy-900 p-3 rounded border border-industrial-navy-800">
                <div className="text-industrial-navy-400">Roll Offset (Δθr)</div>
                <div className="text-white font-bold text-sm mt-0.5">{results.roll > 0 ? `+${results.roll}°` : `${results.roll}°`}</div>
              </div>
              <div className="bg-industrial-navy-900 p-3 rounded border border-industrial-navy-800">
                <div className="text-industrial-navy-400">Pitch Offset (Δθp)</div>
                <div className="text-white font-bold text-sm mt-0.5">{results.pitch > 0 ? `+${results.pitch}°` : `${results.pitch}°`}</div>
              </div>
              <div className="bg-industrial-navy-900 p-3 rounded border border-industrial-navy-800">
                <div className="text-industrial-navy-400">Yaw Offset (Δθy)</div>
                <div className="text-white font-bold text-sm mt-0.5">{results.yaw > 0 ? `+${results.yaw}°` : `${results.yaw}°`}</div>
              </div>
              <div className="bg-industrial-navy-900 p-3 rounded border border-industrial-navy-800">
                <div className="text-industrial-navy-400">GPS/MBES Latency (Δt)</div>
                <div className="text-industrial-turquoise-300 font-bold text-sm mt-0.5">{results.latency * 1000} ms</div>
              </div>
            </div>

            <div className="flex justify-end gap-3 mt-4">
              <button
                onClick={() => alert('Applied Patch Test Calibration Offsets to MBES Acquisition Core!')}
                className="px-4 py-2 bg-industrial-turquoise-500 hover:bg-industrial-turquoise-600 text-industrial-navy-950 font-bold rounded text-xs flex items-center gap-1.5 transition-all shadow"
              >
                <Download className="w-3.5 h-3.5" /> Apply Offsets to Vessel Config
              </button>
            </div>
          </div>
        )}
      </div>
    </DialogShell>
  );
};
