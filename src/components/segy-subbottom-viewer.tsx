import React, { useState } from 'react';
import { DialogShell } from './dialog-shell';
import { Download, Activity } from 'lucide-react';

interface SegySubbottomViewerProps {
  isOpen: boolean;
  onClose: () => void;
}

export const SegySubbottomViewer: React.FC<SegySubbottomViewerProps> = ({ isOpen, onClose }) => {
  const [gain, setGain] = useState(12); // dB
  const [bandpassLow] = useState(500); // Hz
  const [bandpassHigh] = useState(4000); // Hz
  const [fileName] = useState('LINE_001_SBP_CHIRP.sgy');
  const [sampleCount] = useState(1024);
  const [traceCount] = useState(450);

  return (
    <DialogShell
      open={isOpen}
      onClose={onClose}
      title="Sub-Bottom Profiler (SBP) SEG-Y Acoustic Interpreter"
      subtitle="Inspect acoustic reflector horizons, sediment stratification, and sub-surface bedrock depth"
      icon={<Activity className="w-5 h-5 text-industrial-turquoise-400" />}
      maxWidth="max-w-5xl"
    >
      <div className="space-y-6">
        {/* Controls Toolbar */}
        <div className="flex flex-wrap items-center justify-between gap-4 bg-industrial-navy-900/80 p-4 rounded-lg border border-industrial-navy-700">
          <div className="flex items-center gap-3">
            <div className="text-sm font-mono text-white font-bold">{fileName}</div>
            <div className="text-xs font-mono text-industrial-navy-400">
              Traces: <span className="text-industrial-turquoise-300">{traceCount}</span> | Samples/Trace: <span className="text-industrial-turquoise-300">{sampleCount}</span>
            </div>
          </div>

          <div className="flex items-center gap-4 text-xs font-mono">
            <div className="flex items-center gap-2">
              <span className="text-industrial-navy-300">Gain (dB):</span>
              <input
                type="range"
                min="0"
                max="36"
                value={gain}
                onChange={(e) => setGain(Number(e.target.value))}
                className="w-24 accent-industrial-turquoise-400"
              />
              <span className="text-white w-6">{gain}</span>
            </div>

            <div className="flex items-center gap-2">
              <span className="text-industrial-navy-300">Bandpass:</span>
              <span className="text-white">{bandpassLow}-{bandpassHigh} Hz</span>
            </div>
          </div>
        </div>

        {/* Dynamic Acoustic Profile Canvas */}
        <div className="bg-industrial-navy-950 p-4 rounded-xl border border-industrial-navy-800 relative overflow-hidden">
          <div className="flex justify-between text-xs font-mono text-industrial-navy-400 mb-2">
            <span>Fix #001 (Start)</span>
            <span>Sub-Bottom Depth Profile (Two-Way Travel Time ms)</span>
            <span>Fix #450 (End)</span>
          </div>

          {/* SBP Waveform Canvas */}
          <div className="h-64 bg-black rounded border border-industrial-navy-850 relative flex items-end p-2 overflow-hidden">
            {/* Horizontal Grid lines */}
            <div className="absolute inset-0 flex flex-col justify-between pointer-events-none opacity-20">
              <div className="border-b border-industrial-turquoise-500 w-full" />
              <div className="border-b border-industrial-turquoise-500 w-full" />
              <div className="border-b border-industrial-turquoise-500 w-full" />
              <div className="border-b border-industrial-turquoise-500 w-full" />
            </div>

            {/* Horizon Layer Overlay Curves */}
            <svg className="absolute inset-0 w-full h-full pointer-events-none" preserveAspectRatio="none" viewBox="0 0 100 100">
              {/* Seabed Horizon */}
              <path d="M 0 35 Q 25 38 50 32 T 100 36" fill="none" stroke="#2dd4bf" strokeWidth="2" />
              {/* Soft Sediment Reflector */}
              <path d="M 0 55 Q 30 60 60 52 T 100 58" fill="none" stroke="#f59e0b" strokeWidth="1.5" strokeDasharray="3,3" />
              {/* Acoustic Bedrock Horizon */}
              <path d="M 0 80 Q 20 85 50 78 T 100 82" fill="none" stroke="#ef4444" strokeWidth="2" />
            </svg>

            {/* Trace Wiggly Amplitude Sim */}
            <div className="w-full h-full flex items-center justify-between opacity-70">
              {Array.from({ length: 60 }).map((_, i) => (
                <div
                  key={i}
                  className="w-1 bg-industrial-turquoise-400/40 rounded-full"
                  style={{
                    height: `${30 + Math.sin(i * 0.3) * 20 + Math.random() * 15}%`,
                  }}
                />
              ))}
            </div>
          </div>

          {/* Legend */}
          <div className="flex items-center justify-between mt-3 text-xs font-mono text-industrial-navy-300">
            <div className="flex items-center gap-4">
              <span className="flex items-center gap-1.5"><span className="w-3 h-0.5 bg-industrial-turquoise-400" /> Seabed Horizon (Water Bottom)</span>
              <span className="flex items-center gap-1.5"><span className="w-3 h-0.5 bg-amber-400 border-b border-dashed" /> Sediment Reflector #1</span>
              <span className="flex items-center gap-1.5"><span className="w-3 h-0.5 bg-red-500" /> Acoustic Bedrock</span>
            </div>
            <button
              onClick={() => alert('Exporting digitized horizons to GeoPackage...')}
              className="px-3 py-1.5 bg-industrial-turquoise-500/20 hover:bg-industrial-turquoise-500/30 text-industrial-turquoise-300 border border-industrial-turquoise-500/40 rounded flex items-center gap-1.5 transition-colors"
            >
              <Download className="w-3.5 h-3.5" /> Export Horizon Layers
            </button>
          </div>
        </div>
      </div>
    </DialogShell>
  );
};
