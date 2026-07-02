/**
 * Splash Screen
 * Shown on cold start for ~2.5s. Animated theodolite lens rotates as
 * the brand identity establishes. Progress bar fills mining (top) and
 * marine (bottom) of the split lens simultaneously — dual-domain metaphor.
 */

import { useEffect, useState } from "react";
import { BrandLogo } from "@/components/brand-logo";
import { colors, APP_VERSION, APP_BUILD } from "@/lib/tokens";
import { useAppStore } from "@/stores/app-store";

const STAGES = [
  { label: "Loading brand assets", duration: 300 },
  { label: "Initializing Tauri runtime", duration: 400 },
  { label: "Establishing IPC bridge", duration: 350 },
  { label: "Preparing workspace", duration: 300 },
];

export function SplashScreen() {
  const setPhase = useAppStore((s) => s.setPhase);
  const [progress, setProgress] = useState(0);
  const [stageIdx, setStageIdx] = useState(0);

  useEffect(() => {
    let mounted = true;
    let elapsed = 0;
    const total = STAGES.reduce((a, s) => a + s.duration, 0);

    function tickStage(i: number) {
      if (!mounted || i >= STAGES.length) {
        if (mounted) setPhase("modules");
        return;
      }
      setStageIdx(i);
      const stage = STAGES[i];
      const start = performance.now();
      const step = (now: number) => {
        if (!mounted) return;
        const t = Math.min(1, (now - start) / stage.duration);
        elapsed =
          STAGES.slice(0, i).reduce((a, s) => a + s.duration, 0) +
          t * stage.duration;
        setProgress((elapsed / total) * 100);
        if (t < 1) requestAnimationFrame(step);
        else tickStage(i + 1);
      };
      requestAnimationFrame(step);
    }
    tickStage(0);

    return () => {
      mounted = false;
    };
  }, [setPhase]);

  return (
    <div className="relative flex h-full w-full flex-col items-center justify-center overflow-hidden bg-navy-base">
      {/* Subtle survey grid background */}
      <div className="bg-survey-grid absolute inset-0 opacity-30" />

      {/* Radial vignette to focus center */}
      <div
        className="absolute inset-0"
        style={{
          background:
            "radial-gradient(ellipse at center, transparent 0%, rgba(10, 25, 47, 0.85) 70%)",
        }}
      />

      <div className="relative z-10 flex flex-col items-center">
        <BrandLogo size={180} animated showWordmark />

        {/* Version + build */}
        <div className="mt-6 font-mono text-[10px] tracking-wider text-steel-gray">
          v{APP_VERSION} · Build {APP_BUILD}
        </div>

        {/* Progress bar */}
        <div className="mt-8 w-72">
          <div className="mb-2 flex items-center justify-between text-[11px] font-mono">
            <span className="text-steel-light">
              {STAGES[stageIdx]?.label ?? "Ready"}
            </span>
            <span style={{ color: colors.industrialOrange }}>
              {Math.round(progress)}%
            </span>
          </div>
          <div className="h-1 w-full overflow-hidden rounded-full bg-navy-border">
            <div
              className="h-full rounded-full transition-[width] duration-100 ease-out"
              style={{
                width: `${progress}%`,
                background: `linear-gradient(90deg, ${colors.miningYellow} 0%, ${colors.industrialOrange} 50%, ${colors.marineTurquoise} 100%)`,
              }}
            />
          </div>
        </div>

        {/* Footer */}
        <div className="absolute bottom-8 left-0 right-0 text-center text-[10px] tracking-[0.2em] text-steel-gray/60">
          GEODETIC · BATHYMETRIC · INDUSTRIAL
        </div>
      </div>
    </div>
  );
}
