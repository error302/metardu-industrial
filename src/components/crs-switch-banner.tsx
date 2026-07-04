/**
 * CRS Switch Banner — shown when a dropped file's EPSG (detected from
 * GeoTIFF GeoKeyDirectory) differs from the active map CRS.
 *
 * Per ARCHITECTURE.md §8.3 — surveyors need CRS to be transparent.
 * Auto-switching without prompting is dangerous (silent reprojection),
 * so we show a banner with Accept / Dismiss.
 */

import { ArrowRight, X } from "lucide-react";
import { useEffect, useState } from "react";
import { colors } from "@/lib/tokens";
import { useAppStore } from "@/stores/app-store";
import { useSurveyStore } from "@/stores/survey-store";

export function CrsSwitchBanner() {
  const detected = useSurveyStore((s) => s.lastDetectedEpsg);
  const activeEpsg = useAppStore((s) => s.settings.defaultEpsg);
  const updateSettings = useAppStore((s) => s.updateSettings);
  const [dismissed, setDismissed] = useState<string | null>(null);

  // Reset dismissal when a new EPSG is detected
  useEffect(() => {
    if (detected) setDismissed(null);
  }, [detected]);

  if (!detected) return null;
  if (detected === activeEpsg) return null;
  if (dismissed === detected) return null;

  return (
    <div
      className="absolute left-1/2 top-12 z-30 flex -translate-x-1/2 items-center gap-3 rounded-md border px-4 py-2 shadow-lg backdrop-blur"
      style={{
        background: "rgba(10, 25, 47, 0.95)",
        borderColor: `${colors.info}60`,
      }}
    >
      <div className="text-xs">
        <span className="text-steel-light">File CRS detected: </span>
        <span className="font-mono font-semibold" style={{ color: colors.info }}>
          {detected}
        </span>
        <ArrowRight className="mx-2 inline h-3 w-3 text-steel-gray" />
        <span className="text-steel-light">Active CRS: </span>
        <span className="font-mono text-steel-light">{activeEpsg}</span>
      </div>

      <button
        onClick={() => {
          updateSettings({ defaultEpsg: detected });
          setDismissed(detected);
        }}
        className="rounded px-2 py-1 text-[10px] font-semibold uppercase tracking-wider"
        style={{
          background: colors.info,
          color: colors.navyBase,
        }}
      >
        Switch
      </button>
      <button
        onClick={() => setDismissed(detected)}
        className="text-steel-gray hover:text-white"
        title="Dismiss"
      >
        <X className="h-3.5 w-3.5" />
      </button>
    </div>
  );
}
