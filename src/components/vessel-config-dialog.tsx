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
import { Anchor} from "lucide-react";
import { colors, rawColors } from "@/lib/tokens";
import { DialogShell, DialogButton } from "@/components/dialog-shell";

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


  function update(field: keyof VesselConfig, value: string | number) {
    setConfig((c) => ({ ...c, [field]: value }));
    setSaved(false);
  }

return (
    <DialogShell
      open={open}
      onClose={onClose}
      title="Vessel Configuration"
      icon={<Anchor className="h-4 w-4" />}
      iconColor={colors.marineTurquoise}
      maxWidth="max-w-2xl"
      subtitle="Lever-arm offsets"
      footerHint="IMU to transducer to GNSS"
      actions={
        <DialogButton variant="secondary" onClick={onClose}>Close</DialogButton>
      }
    >
          {/* Vessel diagram */}
          <div className="mb-5 rounded-md border border-navy-border bg-navy-base p-4">
            <div className="mb-2 text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
              Vessel Layout (Top-Down View)
            </div>
            <svg viewBox="0 0 400 200" className="w-full" style={{ maxHeight: "180px" }}>
              {/* Vessel hull */}
              <ellipse cx="200" cy="100" rx="120" ry="35" fill="none" stroke={rawColors.steelGray} strokeWidth="1.5" />
              <text x="200" y="105" textAnchor="middle" fill={rawColors.steelGray} fontSize="9" fontFamily="JetBrains Mono">HULL</text>

              {/* Bow direction */}
              <path d="M 320 100 L 340 95 L 340 105 Z" fill={rawColors.steelGray} />
              <text x="345" y="103" fill={rawColors.steelGray} fontSize="8">FWD</text>

              {/* IMU (origin) */}
              <circle cx="200" cy="100" r="5" fill={rawColors.industrialOrange} />
              <text x="200" y="118" textAnchor="middle" fill={rawColors.industrialOrange} fontSize="8" fontWeight="bold">IMU</text>

              {/* Transducer */}
              <circle
                cx={200 + config.transducer_x * 20}
                cy={100 + config.transducer_y * 20}
                r="5" fill={rawColors.marineTurquoise}
              />
              <text
                x={200 + config.transducer_x * 20}
                y={100 + config.transducer_y * 20 - 10}
                textAnchor="middle" fill={rawColors.marineTurquoise} fontSize="8" fontWeight="bold"
              >TXD</text>

              {/* GNSS */}
              <circle
                cx={200 + config.gnss_x * 20}
                cy={100 + config.gnss_y * 20}
                r="5" fill={rawColors.pass}
              />
              <text
                x={200 + config.gnss_x * 20}
                y={100 + config.gnss_y * 20 - 10}
                textAnchor="middle" fill={rawColors.pass} fontSize="8" fontWeight="bold"
              >GNSS</text>

              {/* Lever-arm lines */}
              <line x1="200" y1="100" x2={200 + config.transducer_x * 20} y2={100 + config.transducer_y * 20}
                stroke={rawColors.marineTurquoise} strokeWidth="1" strokeDasharray="3,2" />
              <line x1="200" y1="100" x2={200 + config.gnss_x * 20} y2={100 + config.gnss_y * 20}
                stroke={rawColors.pass} strokeWidth="1" strokeDasharray="3,2" />
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
    </DialogShell>
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
