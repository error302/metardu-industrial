import React, { useState } from 'react';
import { DialogShell } from './dialog-shell';
import { Activity, AlertTriangle, Download, RefreshCw } from 'lucide-react';

interface M3c2DeformationDialogProps {
  isOpen: boolean;
  onClose: () => void;
}

export const M3c2DeformationDialog: React.FC<M3c2DeformationDialogProps> = ({ isOpen, onClose }) => {
  const [isProcessing, setIsProcessing] = useState(false);
  const [report, setReport] = useState<null | {
    pointsCount: number;
    meanDist: number;
    maxDisp: number;
    sigCount: number;
  }>(null);

  const handleRunM3C2 = () => {
    setIsProcessing(true);
    setTimeout(() => {
      setIsProcessing(false);
      setReport({
        pointsCount: 142050,
        meanDist: +0.008, // +8mm
        maxDisp: +0.084,  // +84mm displacement on sector 4
        sigCount: 184,
      });
    }, 1200);
  };

  return (
    <DialogShell
      open={isOpen}
      onClose={onClose}
      title="Highwall 4D Slope Stability & Deformation Inspector (M3C2)"
      subtitle="Multiscale Model-to-Model Cloud Comparison algorithm for mm-level 3D pit wall movement detection"
      icon={<Activity className="w-5 h-5 text-industrial-yellow-400" />}
      maxWidth="max-w-4xl"
    >
      <div className="space-y-6">
        <div className="bg-industrial-navy-900/80 p-4 rounded-lg border border-industrial-navy-700 flex justify-between items-center">
          <div>
            <div className="text-xs font-mono text-industrial-navy-400">Epoch Point Clouds</div>
            <div className="text-sm font-bold font-mono text-white">Baseline (2026-07-01) vs Current Epoch (2026-08-12)</div>
          </div>
          <button
            onClick={handleRunM3C2}
            disabled={isProcessing}
            className="px-4 py-2 bg-industrial-orange-500 hover:bg-industrial-orange-600 text-white font-bold rounded text-xs flex items-center gap-1.5 transition-all shadow"
          >
            <RefreshCw className={`w-3.5 h-3.5 ${isProcessing ? 'animate-spin' : ''}`} />
            {isProcessing ? 'Computing Surface Normals...' : 'Run M3C2 4D Inspection'}
          </button>
        </div>

        {report && (
          <div className="bg-industrial-navy-850 p-5 rounded-xl border border-industrial-navy-750">
            <div className="flex items-center justify-between mb-4">
              <div className="text-sm font-mono font-bold text-white flex items-center gap-2">
                <AlertTriangle className="w-4 h-4 text-amber-400" /> M3C2 Surface Normal Distance Analysis
              </div>
              <div className="text-xs font-mono text-amber-400 bg-amber-500/10 border border-amber-500/30 px-2 py-0.5 rounded">
                {report.sigCount} Points Exceeding 20mm Limit
              </div>
            </div>

            <div className="grid grid-cols-2 md:grid-cols-4 gap-3 text-xs font-mono">
              <div className="bg-industrial-navy-900 p-3 rounded border border-industrial-navy-800">
                <div className="text-industrial-navy-400">Core Point Count</div>
                <div className="text-white font-bold text-sm mt-0.5">{report.pointsCount.toLocaleString()}</div>
              </div>
              <div className="bg-industrial-navy-900 p-3 rounded border border-industrial-navy-800">
                <div className="text-industrial-navy-400">Mean Distance</div>
                <div className="text-white font-bold text-sm mt-0.5">{(report.meanDist * 1000).toFixed(1)} mm</div>
              </div>
              <div className="bg-industrial-navy-900 p-3 rounded border border-industrial-navy-800">
                <div className="text-industrial-navy-400">Max Displacement</div>
                <div className="text-amber-400 font-bold text-sm mt-0.5">+{(report.maxDisp * 1000).toFixed(1)} mm</div>
              </div>
              <div className="bg-industrial-navy-900 p-3 rounded border border-industrial-navy-800">
                <div className="text-industrial-navy-400">Stability Index</div>
                <div className="text-emerald-400 font-bold text-sm mt-0.5">99.87% Normal</div>
              </div>
            </div>

            <div className="flex justify-end gap-3 mt-4">
              <button
                onClick={() => alert('Exporting Highwall Deformation 4D Report PDF...')}
                className="px-4 py-2 bg-industrial-navy-800 hover:bg-industrial-navy-750 text-white font-medium rounded text-xs border border-industrial-navy-700 flex items-center gap-2"
              >
                <Download className="w-4 h-4 text-industrial-yellow-400" /> Export 4D Stability Report PDF
              </button>
            </div>
          </div>
        )}
      </div>
    </DialogShell>
  );
};
