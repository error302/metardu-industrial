import React, { useState, useEffect } from 'react';
import { DialogShell } from './dialog-shell';
import { Navigation, Radio } from 'lucide-react';

interface HelmsmanDisplayProps {
  isOpen: boolean;
  onClose: () => void;
}

export const HelmsmanDisplay: React.FC<HelmsmanDisplayProps> = ({ isOpen, onClose }) => {
  const [lineName] = useState('SOL-LINE-04B');
  const [sog] = useState(4.2); // knots
  const [cog] = useState(184.5); // deg
  const [heading] = useState(185.1); // deg
  const [xte, setXte] = useState(-0.84); // meters (negative = left)
  const [distanceToGo, setDistanceToGo] = useState(420.5); // meters
  const [fixQuality] = useState('RTK Fixed (4)');
  const [satellites] = useState(14);
  const [isSimulating] = useState(true);

  useEffect(() => {
    if (!isSimulating || !isOpen) return;
    const interval = setInterval(() => {
      setXte((prev) => {
        const noise = (Math.random() - 0.5) * 0.1;
        const next = prev + noise;
        return Math.max(-5.0, Math.min(5.0, next));
      });
      setDistanceToGo((prev) => Math.max(0, prev - 2.1));
    }, 500);
    return () => clearInterval(interval);
  }, [isSimulating, isOpen]);

  const xtePercentage = Math.max(-100, Math.min(100, (xte / 5.0) * 100));

  return (
    <DialogShell
      open={isOpen}
      onClose={onClose}
      title="Real-Time Helmsman Navigation & Steering Display"
      subtitle="Live vessel positioning, Cross-Track Error (XTE), and survey line steering indicator"
      icon={<Navigation className="w-5 h-5 text-industrial-turquoise-400" />}
      maxWidth="max-w-4xl"
    >
      <div className="space-y-6">
        {/* Header Indicator */}
        <div className="flex items-center justify-between bg-industrial-navy-900/80 p-4 rounded-lg border border-industrial-navy-700">
          <div>
            <div className="text-xs font-mono text-industrial-navy-400 uppercase tracking-wider">Active Survey Line</div>
            <div className="text-xl font-bold text-white flex items-center gap-2">
              <span>{lineName}</span>
              <span className="text-xs bg-industrial-turquoise-500/20 text-industrial-turquoise-300 border border-industrial-turquoise-500/30 px-2 py-0.5 rounded">
                Acquisition Active
              </span>
            </div>
          </div>
          <div className="flex items-center gap-4 text-sm font-mono">
            <div className="flex items-center gap-1 text-emerald-400">
              <Radio className="w-4 h-4 animate-pulse" />
              <span>{fixQuality}</span>
            </div>
            <div className="text-industrial-navy-300">Sats: <strong className="text-white">{satellites}</strong></div>
          </div>
        </div>

        {/* Steering Bar Gauge */}
        <div className="bg-industrial-navy-900/90 p-6 rounded-xl border border-industrial-navy-700 shadow-inner">
          <div className="flex justify-between items-center mb-2 font-mono text-xs text-industrial-navy-300">
            <span>PORT 5.0m</span>
            <span className="font-bold text-sm text-white">XTE: <span className={Math.abs(xte) > 2.0 ? 'text-amber-400' : 'text-emerald-400'}>{xte >= 0 ? `+${xte.toFixed(2)}m` : `${xte.toFixed(2)}m`}</span></span>
            <span>STBD 5.0m</span>
          </div>

          <div className="relative h-12 bg-industrial-navy-950 rounded-lg overflow-hidden border border-industrial-navy-800 flex items-center justify-center">
            {/* Center target line */}
            <div className="absolute top-0 bottom-0 left-1/2 w-0.5 bg-emerald-500 z-10" />
            <div className="absolute top-0 bottom-0 left-1/4 w-px bg-industrial-navy-800" />
            <div className="absolute top-0 bottom-0 left-3/4 w-px bg-industrial-navy-800" />

            {/* Vessel Position Pointer */}
            <div
              className="absolute transition-all duration-300 transform -translate-x-1/2 flex flex-col items-center"
              style={{ left: `${50 + xtePercentage / 2}%` }}
            >
              <Navigation className="w-8 h-8 text-industrial-turquoise-400 drop-shadow-[0_0_8px_rgba(45,212,191,0.6)]" style={{ transform: `rotate(${heading - cog}deg)` }} />
            </div>
          </div>

          <div className="text-center mt-2 text-xs font-mono text-industrial-navy-400">
            {xte < 0 ? 'STEER STARBOARD ➔' : xte > 0 ? '⬅ STEER PORT' : 'ON LINE'}
          </div>
        </div>

        {/* Readouts Grid */}
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
          <div className="bg-industrial-navy-850 p-4 rounded-lg border border-industrial-navy-750">
            <div className="text-xs text-industrial-navy-400 font-mono">Speed Over Ground</div>
            <div className="text-2xl font-bold text-white font-mono mt-1">{sog.toFixed(1)} <span className="text-xs text-industrial-navy-400">kts</span></div>
          </div>

          <div className="bg-industrial-navy-850 p-4 rounded-lg border border-industrial-navy-750">
            <div className="text-xs text-industrial-navy-400 font-mono">Course Over Ground</div>
            <div className="text-2xl font-bold text-white font-mono mt-1">{cog.toFixed(1)}°</div>
          </div>

          <div className="bg-industrial-navy-850 p-4 rounded-lg border border-industrial-navy-750">
            <div className="text-xs text-industrial-navy-400 font-mono">Gyro Heading</div>
            <div className="text-2xl font-bold text-white font-mono mt-1">{heading.toFixed(1)}°</div>
          </div>

          <div className="bg-industrial-navy-850 p-4 rounded-lg border border-industrial-navy-750">
            <div className="text-xs text-industrial-navy-400 font-mono">Distance-to-Go</div>
            <div className="text-2xl font-bold text-emerald-400 font-mono mt-1">{distanceToGo.toFixed(1)} <span className="text-xs text-industrial-navy-400">m</span></div>
          </div>
        </div>
      </div>
    </DialogShell>
  );
};
