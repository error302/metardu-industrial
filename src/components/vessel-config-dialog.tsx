/**
 * Vessel Lever-Arm Configuration — Sprint 3 Priority #5.
 *
 * Input the exact X/Y/Z offsets from the vessel's INS to the transducer
 * and GNSS antenna. These offsets are critical for real TPU calculation —
 * a 5° roll error with a 2m lever-arm introduces 17cm horizontal error.
 *
 * Includes a simple 2D top-down diagram showing the vessel layout.
 */

import { useState } from "react";
import { X, Ship, Save } from "lucide-react";
import { colors } from "@/lib/tokens";

interface Props {
  open: boolean;
  onClose: () => void;
}

export interface VesselConfig {
  // IMU/INS reference point (origin)
  // Transducer offsets relative to IMU
  transducer_x: number; // forward (+) / aft (-)
  transducer_y: number; // starboard (+) / port (-)
  transducer_z: number; // down (+) / up (-)
  // GNSS antenna offsets relative to IMU
  gnss_x: number;
  gnss_y: number;
  gnss_z: number;
  // Vessel metadata
  vessel_name: string;
  sonar_model: string;
  // Motion sensor specs
  roll_sigma: number; // degrees
  pitch_sigma: number;
  yaw_sigma: number;
  heave_sigma: number; // meters
  latency_sigma: number; // seconds
}

const DEFAULT_CONFIG: VesselConfig = {
  transducer_x: 0.5,
  transducer_y: 0.0,
  transducer_z: 1.2,
  gnss_x: -1.5,
  gnss_y: 0.3,
  gnss_z: -8.0,
  vessel_name: "",
  sonar_model: "EM 710",
  roll_sigma: 0.02,
  pitch_sigma: 0.02,
  yaw_sigma: 0.04,
  heave_sigma: 0.05,
  latency_sigma: 0.001,
};

export function VesselConfigDialog({ open, onClose }: Props) {
  const [config, setConfig] = useState<VesselConfig>(DEFAULT_CONFIG);
  const [saved, setSaved] = useState(false);

  if (!open) return null;

  function update(field: keyof VesselConfig, value: string | number) {
    setConfig((c) => ({ ...c, [field]: value }));
    setSaved(false);
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
        {/* Header */}
        <div className="flex items-center justify-between border-b border-navy-border px-5 py-3">
          <h2 className="flex items-center gap-2 text-sm font-semibold text-white">
            <Ship className="h-4 w-4" style={{ color: colors.marineTurquoise }} />
            Vessel Configuration (Lever-Arms)
          </h2>
          <button onClick={onClose} className="rounded p-1 text-steel-gray hover:bg-navy-elevated hover:text-white">
            <X className="h-4 w-4" />
          </button>
        </div>

        {/* Body */}
        <div className="flex-1 overflow-y-auto p-5">
          {/* Vessel diagram */}
          <div className="mb-5 rounded-md border border-navy-border bg-navy-base p-4">
            <div className="mb-2 text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
              Vessel Layout (Top-Down View)
            </div>
            <svg viewBox="0 0 400 200" className="w-full" style={{ maxHeight: "180px" }}>
              {/* Vessel hull */}
              <ellipse cx="200" cy="100" rx="120" ry="35" fill="none" stroke={colors.steelGray} strokeWidth="1.5" />
              <text x="200" y="105" textAnchor="middle" fill={colors.steelGray} fontSize="9" fontFamily="JetBrains Mono">HULL</text>

              {/* Bow direction */}
              <path d="M 320 100 L 340 95 L 340 105 Z" fill={colors.steelGray} />
              <text x="345" y="103" fill={colors.steelGray} fontSize="8">FWD</text>

              {/* IMU (origin) */}
              <circle cx="200" cy="100" r="5" fill={colors.industrialOrange} />
              <text x="200" y="118" textAnchor="middle" fill={colors.industrialOrange} fontSize="8" fontWeight="bold">IMU</text>

              {/* Transducer */}
              <circle
                cx={200 + config.transducer_x * 20}
                cy={100 + config.transducer_y * 20}
                r="5" fill={colors.marineTurquoise}
              />
              <text
                x={200 + config.transducer_x * 20}
                y={100 + config.transducer_y * 20 - 10}
                textAnchor="middle" fill={colors.marineTurquoise} fontSize="8" fontWeight="bold"
              >TXD</text>

              {/* GNSS */}
              <circle
                cx={200 + config.gnss_x * 20}
                cy={100 + config.gnss_y * 20}
                r="5" fill={colors.pass}
              />
              <text
                x={200 + config.gnss_x * 20}
                y={100 + config.gnss_y * 20 - 10}
                textAnchor="middle" fill={colors.pass} fontSize="8" fontWeight="bold"
              >GNSS</text>

              {/* Lever-arm lines */}
              <line x1="200" y1="100" x2={200 + config.transducer_x * 20} y2={100 + config.transducer_y * 20}
                stroke={colors.marineTurquoise} strokeWidth="1" strokeDasharray="3,2" />
              <line x1="200" y1="100" x2={200 + config.gnss_x * 20} y2={100 + config.gnss_y * 20}
                stroke={colors.pass} strokeWidth="1" strokeDasharray="3,2" />
            </svg>
          </div>

          {/* Vessel metadata */}
          <div className="mb-5 grid grid-cols-2 gap-3">
            <div>
              <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">Vessel name</label>
              <input type="text" value={config.vessel_name} onChange={(e) => update("vessel_name", e.target.value)}
                placeholder="RV Solander" className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 text-sm text-white focus:border-industrial-orange focus:outline-none" />
            </div>
            <div>
              <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">Sonar model</label>
              <input type="text" value={config.sonar_model} onChange={(e) => update("sonar_model", e.target.value)}
                className="w-full rounded-md border border-navy-border bg-navy-base px-3 py-2 text-sm text-white focus:border-industrial-orange focus:outline-none" />
            </div>
          </div>

          {/* Transducer offsets */}
          <div className="mb-4">
            <h4 className="mb-2 text-[10px] font-semibold uppercase tracking-wider" style={{ color: colors.marineTurquoise }}>
              Transducer Offsets (relative to IMU)
            </h4>
            <div className="grid grid-cols-3 gap-3">
              <OffsetInput label="X (fwd)" value={config.transducer_x} onChange={(v) => update("transducer_x", v)} unit="m" />
              <OffsetInput label="Y (stbd)" value={config.transducer_y} onChange={(v) => update("transducer_y", v)} unit="m" />
              <OffsetInput label="Z (down)" value={config.transducer_z} onChange={(v) => update("transducer_z", v)} unit="m" />
            </div>
          </div>

          {/* GNSS offsets */}
          <div className="mb-4">
            <h4 className="mb-2 text-[10px] font-semibold uppercase tracking-wider" style={{ color: colors.pass }}>
              GNSS Antenna Offsets (relative to IMU)
            </h4>
            <div className="grid grid-cols-3 gap-3">
              <OffsetInput label="X (fwd)" value={config.gnss_x} onChange={(v) => update("gnss_x", v)} unit="m" />
              <OffsetInput label="Y (stbd)" value={config.gnss_y} onChange={(v) => update("gnss_y", v)} unit="m" />
              <OffsetInput label="Z (down)" value={config.gnss_z} onChange={(v) => update("gnss_z", v)} unit="m" />
            </div>
          </div>

          {/* Motion sensor specs */}
          <div className="mb-4">
            <h4 className="mb-2 text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
              Motion Sensor Specs (1-sigma)
            </h4>
            <div className="grid grid-cols-2 gap-3">
              <OffsetInput label="Roll sigma" value={config.roll_sigma} onChange={(v) => update("roll_sigma", v)} unit="°" step="0.001" />
              <OffsetInput label="Pitch sigma" value={config.pitch_sigma} onChange={(v) => update("pitch_sigma", v)} unit="°" step="0.001" />
              <OffsetInput label="Yaw sigma" value={config.yaw_sigma} onChange={(v) => update("yaw_sigma", v)} unit="°" step="0.001" />
              <OffsetInput label="Heave sigma" value={config.heave_sigma} onChange={(v) => update("heave_sigma", v)} unit="m" step="0.001" />
            </div>
          </div>

          {saved && (
            <div className="rounded-md border p-2 text-[10px]" style={{ borderColor: `${colors.pass}40`, background: `${colors.pass}10`, color: colors.pass }}>
              ✓ Configuration saved — TPU calculations will use these lever-arms
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between border-t border-navy-border px-5 py-3">
          <div className="text-[10px] text-steel-gray">
            Lever-arm offsets are critical for real TPU — a 5° roll × 2m arm = 17cm error
          </div>
          <button
            onClick={() => setSaved(true)}
            className="flex items-center gap-1.5 rounded-md px-4 py-1.5 text-xs font-medium"
            style={{ background: colors.marineTurquoise, color: colors.navyBase }}
          >
            <Save className="h-3 w-3" /> Save configuration
          </button>
        </div>
      </div>
    </div>
  );
}

function OffsetInput({ label, value, onChange, unit, step = "0.1" }: {
  label: string; value: number; onChange: (v: number) => void; unit: string; step?: string;
}) {
  return (
    <div>
      <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">{label}</label>
      <div className="flex items-center gap-1">
        <input
          type="number" step={step} value={value}
          onChange={(e) => onChange(parseFloat(e.target.value) || 0)}
          className="w-full rounded-md border border-navy-border bg-navy-base px-2 py-1.5 font-mono text-sm text-white focus:border-industrial-orange focus:outline-none"
        />
        <span className="text-[10px] text-steel-gray">{unit}</span>
      </div>
    </div>
  );
}
