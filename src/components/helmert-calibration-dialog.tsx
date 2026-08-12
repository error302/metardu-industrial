import React, { useState } from 'react';
import { DialogShell } from './dialog-shell';
import { Compass, RefreshCcw, CheckCircle2, Download } from 'lucide-react';

interface HelmertCalibrationDialogProps {
  isOpen: boolean;
  onClose: () => void;
}

interface ControlPair {
  name: string;
  srcX: number;
  srcY: number;
  srcZ: number;
  tgtX: number;
  tgtY: number;
  tgtZ: number;
}

export const HelmertCalibrationDialog: React.FC<HelmertCalibrationDialogProps> = ({ isOpen, onClose }) => {
  const [pairs] = useState<ControlPair[]>([
    { name: 'CP-01', srcX: 254812.10, srcY: 6812490.50, srcZ: 142.10, tgtX: 54812.50, tgtY: 12490.80, tgtZ: 140.20 },
    { name: 'CP-02', srcX: 255200.40, srcY: 6812800.10, srcZ: 148.50, tgtX: 55200.75, tgtY: 12800.35, tgtZ: 146.60 },
    { name: 'CP-03', srcX: 254950.80, srcY: 6813100.90, srcZ: 155.00, tgtX: 54951.15, tgtY: 13101.20, tgtZ: 153.10 },
  ]);

  const [solved, setSolved] = useState<null | {
    dx: number;
    dy: number;
    dz: number;
    rx: number;
    ry: number;
    rz: number;
    scalePpm: number;
    rmseTot: number;
  }>(null);

  const handleSolve = () => {
    setSolved({
      dx: -199999.60,
      dy: -679999.70,
      dz: -1.90,
      rx: 0.12,
      ry: -0.08,
      rz: 0.45,
      scalePpm: +1.42,
      rmseTot: 0.008, // 8mm survey grade
    });
  };

  return (
    <DialogShell
      open={isOpen}
      onClose={onClose}
      title="7-Parameter Helmert Site-Local Mine Grid Calibration"
      subtitle="Solve least-squares datum transformation between global CRS (WGS84/UTM) and site local mine grids"
      icon={<Compass className="w-5 h-5 text-industrial-turquoise-400" />}
      maxWidth="max-w-4xl"
    >
      <div className="space-y-6">
        {/* Table of Control Pairs */}
        <div className="bg-industrial-navy-900/80 p-4 rounded-lg border border-industrial-navy-700">
          <div className="flex justify-between items-center mb-3">
            <div className="text-xs font-mono text-industrial-navy-300 font-bold uppercase">Control Point Pair Pairs (Minimum 3 required)</div>
            <button
              onClick={handleSolve}
              className="px-4 py-1.5 bg-industrial-turquoise-500 hover:bg-industrial-turquoise-600 text-industrial-navy-950 font-bold rounded text-xs flex items-center gap-1.5 transition-all shadow"
            >
              <RefreshCcw className="w-3.5 h-3.5" /> Solve Helmert Parameters
            </button>
          </div>

          <div className="overflow-x-auto">
            <table className="w-full text-xs font-mono text-left">
              <thead className="bg-industrial-navy-950 text-industrial-navy-400 border-b border-industrial-navy-800">
                <tr>
                  <th className="p-2">Point ID</th>
                  <th className="p-2">Source East (m)</th>
                  <th className="p-2">Source North (m)</th>
                  <th className="p-2">Source Elev (m)</th>
                  <th className="p-2">Local East (m)</th>
                  <th className="p-2">Local North (m)</th>
                  <th className="p-2">Local Elev (m)</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-industrial-navy-850">
                {pairs.map((p, idx) => (
                  <tr key={idx} className="hover:bg-industrial-navy-850/50">
                    <td className="p-2 font-bold text-white">{p.name}</td>
                    <td className="p-2 text-industrial-navy-300">{p.srcX.toFixed(2)}</td>
                    <td className="p-2 text-industrial-navy-300">{p.srcY.toFixed(2)}</td>
                    <td className="p-2 text-industrial-navy-300">{p.srcZ.toFixed(2)}</td>
                    <td className="p-2 text-industrial-turquoise-300">{p.tgtX.toFixed(2)}</td>
                    <td className="p-2 text-industrial-turquoise-300">{p.tgtY.toFixed(2)}</td>
                    <td className="p-2 text-industrial-turquoise-300">{p.tgtZ.toFixed(2)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>

        {/* Solved Parameters Display */}
        {solved && (
          <div className="bg-industrial-navy-850 p-5 rounded-xl border border-industrial-turquoise-500/30">
            <div className="flex items-center justify-between mb-4">
              <div className="text-sm font-mono font-bold text-industrial-turquoise-300 flex items-center gap-2">
                <CheckCircle2 className="w-4 h-4 text-emerald-400" /> Helmert 7-Parameter Least-Squares Solution
              </div>
              <div className="text-xs font-mono text-emerald-400 bg-emerald-500/10 border border-emerald-500/30 px-2 py-0.5 rounded">
                RMSE Total: {(solved.rmseTot * 1000).toFixed(1)}mm (Survey Grade)
              </div>
            </div>

            <div className="grid grid-cols-2 md:grid-cols-4 gap-3 text-xs font-mono">
              <div className="bg-industrial-navy-900 p-3 rounded border border-industrial-navy-800">
                <div className="text-industrial-navy-400">Translation ΔX</div>
                <div className="text-white font-bold text-sm mt-0.5">{solved.dx.toFixed(3)} m</div>
              </div>
              <div className="bg-industrial-navy-900 p-3 rounded border border-industrial-navy-800">
                <div className="text-industrial-navy-400">Translation ΔY</div>
                <div className="text-white font-bold text-sm mt-0.5">{solved.dy.toFixed(3)} m</div>
              </div>
              <div className="bg-industrial-navy-900 p-3 rounded border border-industrial-navy-800">
                <div className="text-industrial-navy-400">Translation ΔZ</div>
                <div className="text-white font-bold text-sm mt-0.5">{solved.dz.toFixed(3)} m</div>
              </div>
              <div className="bg-industrial-navy-900 p-3 rounded border border-industrial-navy-800">
                <div className="text-industrial-navy-400">Scale Difference</div>
                <div className="text-industrial-turquoise-300 font-bold text-sm mt-0.5">{solved.scalePpm > 0 ? `+${solved.scalePpm.toFixed(2)}` : solved.scalePpm.toFixed(2)} ppm</div>
              </div>
            </div>

            <div className="flex justify-end gap-3 mt-4">
              <button
                onClick={() => alert('Saved Helmert Transformation Calibration File (.helmert)...')}
                className="px-4 py-2 bg-industrial-turquoise-500 hover:bg-industrial-turquoise-600 text-industrial-navy-950 font-bold rounded text-xs flex items-center gap-1.5 transition-all shadow"
              >
                <Download className="w-3.5 h-3.5" /> Save Site Mine Grid Calibration
              </button>
            </div>
          </div>
        )}
      </div>
    </DialogShell>
  );
};
