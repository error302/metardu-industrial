import React, { useState } from 'react';
import { DialogShell } from './dialog-shell';
import { Edit3, Square, Circle, Hash, Download, MousePointer } from 'lucide-react';

interface CadVectorEditorProps {
  isOpen: boolean;
  onClose: () => void;
}

export const CadVectorEditor: React.FC<CadVectorEditorProps> = ({ isOpen, onClose }) => {
  const [activeTool, setActiveTool] = useState<'select' | 'point' | 'line' | 'polyline' | '3dface'>('select');
  const [layerName] = useState('DESIGN_BOUNDARY_01');
  const [entityCount] = useState(128);

  return (
    <DialogShell
      open={isOpen}
      onClose={onClose}
      title="Interactive Vector CAD Drawing & DXF Editor"
      subtitle="Create, edit, digitize, and transform survey vectors, polylines, 3D faces, and CAD layers"
      icon={<Edit3 className="w-5 h-5 text-industrial-yellow-400" />}
      maxWidth="max-w-5xl"
    >
      <div className="space-y-6">
        {/* CAD Toolbar */}
        <div className="flex flex-wrap items-center justify-between gap-4 bg-industrial-navy-900/80 p-3 rounded-lg border border-industrial-navy-700">
          <div className="flex items-center gap-2">
            <button
              onClick={() => setActiveTool('select')}
              className={`p-2 rounded font-mono text-xs flex items-center gap-1.5 transition-colors ${
                activeTool === 'select'
                  ? 'bg-industrial-orange-500 text-white font-bold'
                  : 'bg-industrial-navy-800 text-industrial-navy-300 hover:text-white'
              }`}
            >
              <MousePointer className="w-4 h-4" /> Select
            </button>
            <button
              onClick={() => setActiveTool('point')}
              className={`p-2 rounded font-mono text-xs flex items-center gap-1.5 transition-colors ${
                activeTool === 'point'
                  ? 'bg-industrial-orange-500 text-white font-bold'
                  : 'bg-industrial-navy-800 text-industrial-navy-300 hover:text-white'
              }`}
            >
              <Circle className="w-4 h-4" /> Point
            </button>
            <button
              onClick={() => setActiveTool('line')}
              className={`p-2 rounded font-mono text-xs flex items-center gap-1.5 transition-colors ${
                activeTool === 'line'
                  ? 'bg-industrial-orange-500 text-white font-bold'
                  : 'bg-industrial-navy-800 text-industrial-navy-300 hover:text-white'
              }`}
            >
              <Hash className="w-4 h-4" /> Line
            </button>
            <button
              onClick={() => setActiveTool('polyline')}
              className={`p-2 rounded font-mono text-xs flex items-center gap-1.5 transition-colors ${
                activeTool === 'polyline'
                  ? 'bg-industrial-orange-500 text-white font-bold'
                  : 'bg-industrial-navy-800 text-industrial-navy-300 hover:text-white'
              }`}
            >
              <Square className="w-4 h-4" /> Polyline
            </button>
          </div>

          <div className="flex items-center gap-4 text-xs font-mono">
            <div className="text-industrial-navy-300">
              Layer: <span className="text-industrial-yellow-400 font-bold">{layerName}</span>
            </div>
            <div className="text-industrial-navy-300">
              Entities: <span className="text-white font-bold">{entityCount}</span>
            </div>
            <button
              onClick={() => alert('Exporting DXF file...')}
              className="px-3 py-1.5 bg-industrial-turquoise-500/20 hover:bg-industrial-turquoise-500/30 text-industrial-turquoise-300 border border-industrial-turquoise-500/40 rounded flex items-center gap-1.5"
            >
              <Download className="w-3.5 h-3.5" /> Export DXF
            </button>
          </div>
        </div>

        {/* CAD Canvas Viewport */}
        <div className="bg-industrial-navy-950 p-4 rounded-xl border border-industrial-navy-800 relative h-72 flex flex-col justify-between overflow-hidden">
          <div className="absolute inset-0 bg-[radial-gradient(#334155_1px,transparent_1px)] [background-size:16px_16px] opacity-30 pointer-events-none" />

          <div className="flex justify-between text-[11px] font-mono text-industrial-navy-400 relative z-10">
            <span>GRID SNAP: 1.0m | ORTHO: ON</span>
            <span>X: 254812.45m | Y: 6812490.12m | Z: 142.50m</span>
          </div>

          {/* Drawn CAD Vectors SVG */}
          <svg className="absolute inset-0 w-full h-full pointer-events-none" viewBox="0 0 500 300">
            <polygon points="100,80 350,60 420,220 150,240" fill="rgba(245,158,11,0.08)" stroke="#f59e0b" strokeWidth="2" />
            <line x1="100" y1="80" x2="420" y2="220" stroke="#2dd4bf" strokeWidth="1.5" strokeDasharray="4,4" />
            <circle cx="100" cy="80" r="4" fill="#ef4444" />
            <circle cx="350" cy="60" r="4" fill="#ef4444" />
            <circle cx="420" cy="220" r="4" fill="#ef4444" />
            <circle cx="150" cy="240" r="4" fill="#ef4444" />
          </svg>

          <div className="relative z-10 flex justify-between items-end">
            <div className="text-xs font-mono bg-industrial-navy-900/90 border border-industrial-navy-750 p-2 rounded text-industrial-navy-300">
              Active Tool: <span className="text-industrial-orange-400 font-bold uppercase">{activeTool}</span> (Click canvas to digitize vertex)
            </div>
          </div>
        </div>
      </div>
    </DialogShell>
  );
};
