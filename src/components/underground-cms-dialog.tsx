import React, { useState } from 'react';
import { DialogShell } from './dialog-shell';
import { Box, AlertCircle, CheckCircle2, Download, RefreshCw } from 'lucide-react';

interface UndergroundCmsDialogProps {
  isOpen: boolean;
  onClose: () => void;
}

export const UndergroundCmsDialog: React.FC<UndergroundCmsDialogProps> = ({ isOpen, onClose }) => {
  const [stopeName] = useState('STOPE-420-SOUTH');
  const [isAnalyzing, setIsAnalyzing] = useState(false);
  const [result] = useState({
    scanPoints: 84520,
    designVol: 14250.0, // m3
    actualVol: 15120.0, // m3
    overbreakVol: 1120.0, // m3
    underbreakVol: 250.0, // m3
    maxOverbreak: 2.14, // m
    maxUnderbreak: 0.85, // m
    netVolChangePct: +6.1,
  });

  const handleRunAnalysis = () => {
    setIsAnalyzing(true);
    setTimeout(() => {
      setIsAnalyzing(false);
    }, 1200);
  };

  return (
    <DialogShell
      open={isOpen}
      onClose={onClose}
      title="Underground CMS Cavity & Stope Overbreak/Underbreak Profiler"
      subtitle="3D laser scan vs design envelope comparison for underground stopes, drives, and tunnels"
      icon={<Box className="w-5 h-5 text-industrial-yellow-400" />}
      maxWidth="max-w-4xl"
    >
      <div className="space-y-6">
        {/* Header Controls */}
        <div className="flex items-center justify-between bg-industrial-navy-900/80 p-4 rounded-lg border border-industrial-navy-700">
          <div>
            <div className="text-xs font-mono text-industrial-navy-400">Target Design Solid</div>
            <div className="text-lg font-bold text-white font-mono">{stopeName}</div>
          </div>

          <button
            onClick={handleRunAnalysis}
            disabled={isAnalyzing}
            className="px-4 py-2 bg-industrial-orange-500 hover:bg-industrial-orange-600 text-white font-bold rounded-lg text-sm flex items-center gap-2 transition-all shadow-md"
          >
            <RefreshCw className={`w-4 h-4 ${isAnalyzing ? 'animate-spin' : ''}`} />
            {isAnalyzing ? 'Processing Scan Mesh...' : 'Run Cavity Analysis'}
          </button>
        </div>

        {/* Results Metrics Grid */}
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
          <div className="bg-industrial-navy-850 p-4 rounded-lg border border-industrial-navy-750">
            <div className="text-xs font-mono text-industrial-navy-400">Design Volume</div>
            <div className="text-xl font-bold text-white font-mono mt-1">{result.designVol.toLocaleString()} <span className="text-xs text-industrial-navy-400">m³</span></div>
          </div>

          <div className="bg-industrial-navy-850 p-4 rounded-lg border border-industrial-navy-750">
            <div className="text-xs font-mono text-industrial-navy-400">As-Scanned Volume</div>
            <div className="text-xl font-bold text-white font-mono mt-1">{result.actualVol.toLocaleString()} <span className="text-xs text-industrial-navy-400">m³</span></div>
          </div>

          <div className="bg-industrial-navy-850 p-4 rounded-lg border border-red-500/30 bg-red-950/10">
            <div className="text-xs font-mono text-red-400 flex items-center gap-1">
              <AlertCircle className="w-3.5 h-3.5" /> Overbreak (Unsafe)
            </div>
            <div className="text-xl font-bold text-red-400 font-mono mt-1">+{result.overbreakVol.toLocaleString()} <span className="text-xs">m³</span></div>
            <div className="text-[10px] font-mono text-industrial-navy-400 mt-1">Max dist: {result.maxOverbreak}m</div>
          </div>

          <div className="bg-industrial-navy-850 p-4 rounded-lg border border-sky-500/30 bg-sky-950/10">
            <div className="text-xs font-mono text-sky-400 flex items-center gap-1">
              <CheckCircle2 className="w-3.5 h-3.5" /> Underbreak (Tight)
            </div>
            <div className="text-xl font-bold text-sky-400 font-mono mt-1">-{result.underbreakVol.toLocaleString()} <span className="text-xs">m³</span></div>
            <div className="text-[10px] font-mono text-industrial-navy-400 mt-1">Max dist: {result.maxUnderbreak}m</div>
          </div>
        </div>

        {/* 3D Stope Mesh Heatmap Visualization Mock */}
        <div className="bg-industrial-navy-950 p-6 rounded-xl border border-industrial-navy-800 text-center relative overflow-hidden h-56 flex flex-col items-center justify-center">
          <div className="text-xs font-mono text-industrial-navy-400 mb-2">3D Cavity Distance Heatmap (Red = Overbreak | Blue = Underbreak)</div>
          
          <div className="relative w-48 h-32 border-2 border-dashed border-industrial-navy-700 rounded-lg flex items-center justify-center bg-industrial-navy-900/50">
            <div className="w-36 h-24 bg-gradient-to-r from-red-500/30 via-emerald-500/20 to-sky-500/30 rounded blur-sm transform -rotate-6" />
            <div className="absolute inset-0 flex items-center justify-center font-mono text-xs text-white font-bold">
              3D Mesh Viewport
            </div>
          </div>

          <div className="flex items-center gap-6 mt-4 text-xs font-mono">
            <span className="flex items-center gap-1 text-red-400"><span className="w-3 h-3 bg-red-500 rounded" /> Overbreak (&gt;+0.5m)</span>
            <span className="flex items-center gap-1 text-emerald-400"><span className="w-3 h-3 bg-emerald-500 rounded" /> On Design (±0.2m)</span>
            <span className="flex items-center gap-1 text-sky-400"><span className="w-3 h-3 bg-sky-500 rounded" /> Underbreak (&lt;-0.5m)</span>
          </div>
        </div>

        {/* Footer Actions */}
        <div className="flex justify-end gap-3 pt-2">
          <button
            onClick={() => alert('Exporting CMS Reconciliation PDF Audit Report...')}
            className="px-4 py-2 bg-industrial-navy-800 hover:bg-industrial-navy-750 text-white font-medium rounded-lg text-sm border border-industrial-navy-700 flex items-center gap-2"
          >
            <Download className="w-4 h-4 text-industrial-yellow-400" /> Export Signed Audit PDF
          </button>
        </div>
      </div>
    </DialogShell>
  );
};
