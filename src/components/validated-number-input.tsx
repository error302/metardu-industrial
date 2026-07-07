/**
 * ValidatedNumberInput — number input with range validation (Sprint 14).
 *
 * Wires the QC `range_checks.rs` module to number inputs so that out-of-
 * range values show a red border + error message on blur. This was the
 * #3 friction point in the UX Researcher audit — 24 number inputs lacked
 * step/min/max, letting typos propagate silently.
 *
 * Validation types:
 *   - "lat" — latitude [-90, 90]
 *   - "lon" — longitude [-180, 180]
 *   - "bearing" — [0, 360)
 *   - "distance" — [min, max] configurable (default 0-100000m)
 *   - "elevation" — [regional_msl - max_dev, regional_msl + max_dev]
 *   - "volume" — [-max, max] (default 1e9 m³)
 *   - "positive" — [0, infinity)
 *   - "custom" — caller provides min/max
 *
 * Usage:
 * ```tsx
 * <ValidatedNumberInput
 *   value={cellSize}
 *   onChange={setCellSize}
 *   validationType="positive"
 *   step={0.1}
 *   label="Cell size (m)"
 * />
 * ```
 */

import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { colors } from "@/lib/tokens";
import { isNative } from "@/lib/tauri-ipc";
import { Tooltip } from "@/components/tooltip";
import { AlertCircle, CheckCircle2 } from "lucide-react";

type ValidationType = "lat" | "lon" | "bearing" | "distance" | "elevation" | "volume" | "positive" | "custom";

interface ValidatedNumberInputProps {
  value: string;
  onChange: (value: string) => void;
  /** Type of validation to apply. */
  validationType?: ValidationType;
  /** For "distance": min value (default 0). For "custom": min. */
  min?: number;
  /** For "distance": max value (default 100000). For "custom": max. */
  max?: number;
  /** For "elevation": regional MSL (default 0). */
  regionalMsl?: number;
  /** For "elevation": max deviation from MSL (default 2000). */
  maxDeviation?: number;
  /** For "volume": max absolute value (default 1e9). */
  maxVolume?: number;
  /** HTML step attribute. */
  step?: number;
  /** Label shown above the input. */
  label?: string;
  /** Placeholder text. */
  placeholder?: string;
  /** Additional className. */
  className?: string;
  /** If true, validate on every change (not just blur). Default: false. */
  validateOnChange?: boolean;
}

interface RangeCheckResult {
  passed: boolean;
  message: string;
  value: number;
  min: number;
  max: number;
}

export function ValidatedNumberInput({
  value,
  onChange,
  validationType = "positive",
  min = 0,
  max = 100000,
  regionalMsl = 0,
  maxDeviation = 2000,
  maxVolume = 1e9,
  step,
  label,
  placeholder,
  className = "",
  validateOnChange = false,
}: ValidatedNumberInputProps) {
  const [error, setError] = useState<string | null>(null);
  const [touched, setTouched] = useState(false);
  const lastValidatedRef = useRef<string>("");

  async function validate(val: string) {
    if (!val.trim()) {
      setError(null);
      return;
    }
    const num = parseFloat(val);
    if (Number.isNaN(num)) {
      setError("Not a valid number");
      return;
    }
    if (!isNative()) {
      // Browser mode — do client-side validation only
      clientSideValidate(num);
      return;
    }
    try {
      let result: RangeCheckResult | null = null;
      switch (validationType) {
        case "lat":
        case "lon": {
          const [latCheck, lonCheck] = await invoke<[RangeCheckResult, RangeCheckResult]>(
            "check_lat_lon_cmd",
            validationType === "lat" ? { lat: num, lon: 0 } : { lat: 0, lon: num },
          );
          result = validationType === "lat" ? latCheck : lonCheck;
          break;
        }
        case "bearing":
          result = await invoke<RangeCheckResult>("check_bearing_cmd", { bearingDeg: num });
          break;
        case "distance":
          result = await invoke<RangeCheckResult>("check_distance_cmd", { distanceM: num, minM: min, maxM: max });
          break;
        case "elevation":
          result = await invoke<RangeCheckResult>("check_elevation_cmd", { elevM: num, regionalMsl: regionalMsl, maxDeviationM: maxDeviation });
          break;
        case "volume":
          result = await invoke<RangeCheckResult>("check_volume_cmd", { volumeM3: num, maxM3: maxVolume });
          break;
        case "positive":
          if (num < 0) {
            setError("Value must be positive");
          } else {
            setError(null);
          }
          return;
        case "custom":
          if (num < min || num > max) {
            setError(`Out of range [{min}, {max}]`);
          } else {
            setError(null);
          }
          return;
      }
      if (result) {
        setError(result.passed ? null : result.message);
      }
    } catch {
      // IPC failed — fall back to client-side
      clientSideValidate(num);
    }
  }

  function clientSideValidate(num: number) {
    let lo = -Infinity, hi = Infinity;
    switch (validationType) {
      case "lat": lo = -90; hi = 90; break;
      case "lon": lo = -180; hi = 180; break;
      case "bearing": lo = 0; hi = 360; break;
      case "distance": lo = min; hi = max; break;
      case "elevation": lo = regionalMsl - maxDeviation; hi = regionalMsl + maxDeviation; break;
      case "volume": lo = -maxVolume; hi = maxVolume; break;
      case "positive": lo = 0; hi = Infinity; break;
      case "custom": lo = min; hi = max; break;
    }
    if (num < lo || num > hi || Number.isNaN(num)) {
      setError(`Out of range [${lo}, ${hi}]`);
    } else {
      setError(null);
    }
  }

  useEffect(() => {
    if (validateOnChange && value !== lastValidatedRef.current) {
      lastValidatedRef.current = value;
      validate(value);
    }
  }, [value, validateOnChange]);

  const showError = touched && error != null;
  const showValid = touched && error == null && value.trim() !== "";

  return (
    <div className={className}>
      {label && (
        <label className="mb-0.5 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
          {label}
        </label>
      )}
      <div className="relative">
        <input
          type="number"
          value={value}
          onChange={(e) => onChange(e.target.value)}
          onBlur={() => {
            setTouched(true);
            validate(value);
          }}
          step={step}
          placeholder={placeholder}
          className="w-full rounded-md border bg-navy-base px-2 py-1.5 font-mono text-sm text-white focus:outline-none"
          style={{
            borderColor: showError ? colors.fail : showValid ? colors.pass : colors.navyBorder,
          }}
        />
        {showError && (
          <div className="absolute right-2 top-1/2 -translate-y-1/2">
            <Tooltip text={error!} position="left" delay={0}>
              <AlertCircle className="h-3.5 w-3.5" style={{ color: colors.fail }} />
            </Tooltip>
          </div>
        )}
        {showValid && (
          <div className="absolute right-2 top-1/2 -translate-y-1/2">
            <CheckCircle2 className="h-3.5 w-3.5" style={{ color: colors.pass }} />
          </div>
        )}
      </div>
      {showError && (
        <div className="mt-0.5 text-[10px]" style={{ color: colors.fail }}>
          {error}
        </div>
      )}
    </div>
  );
}
