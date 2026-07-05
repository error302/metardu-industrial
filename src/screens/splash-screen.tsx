/**
 * Splash Screen — Professional Edition
 *
 * Redesigned for a polished, enterprise-grade first impression:
 *   - Animated SVG theodolite reticle (survey instrument metaphor)
 *   - Glassmorphic card with subtle border glow
 *   - Stepped progress with monospace stage labels
 *   - Subtle animated grid + scanline background
 *   - Clean typographic hierarchy
 *   - Dual-domain accent (mining yellow → industrial orange → marine teal)
 *
 * The splash runs for ~2.5s on cold start, establishing brand identity
 * before the module loading screen takes over.
 */

import { useEffect, useState } from "react";
import { colors, rawColors, APP_VERSION, APP_BUILD, APP_NAME } from "@/lib/tokens";
import { useAppStore } from "@/stores/app-store";
import { useViewport } from "@/lib/use-viewport";

const STAGES = [
  { label: "Initializing runtime", duration: 350 },
  { label: "Establishing IPC bridge", duration: 400 },
  { label: "Loading geospatial core", duration: 350 },
  { label: "Preparing workspace", duration: 300 },
];

export function SplashScreen() {
  const setPhase = useAppStore((s) => s.setPhase);
  const [progress, setProgress] = useState(0);
  const [stageIdx, setStageIdx] = useState(0);
  const { isNarrow, isVeryNarrow } = useViewport();

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

  const logoSize = isVeryNarrow ? 100 : isNarrow ? 130 : 160;
  const cardWidth = isVeryNarrow ? 300 : isNarrow ? 340 : 380;

  return (
    <div
      className="relative flex h-full w-full items-center justify-center overflow-hidden"
      style={{ background: colors.navyBase }}
    >
      {/* ── Background layers ── */}
      {/* Animated grid */}
      <div className="absolute inset-0 opacity-[0.15]">
        <div
          className="absolute inset-0"
          style={{
            backgroundImage: `
              linear-gradient(${colors.navyBorder} 1px, transparent 1px),
              linear-gradient(90deg, ${colors.navyBorder} 1px, transparent 1px)
            `,
            backgroundSize: "40px 40px",
            maskImage:
              "radial-gradient(ellipse at center, black 0%, transparent 70%)",
            WebkitMaskImage:
              "radial-gradient(ellipse at center, black 0%, transparent 70%)",
          }}
        />
      </div>

      {/* Slow horizontal scanline */}
      <div
        className="absolute inset-x-0 h-px opacity-40"
        style={{
          background: `linear-gradient(90deg, transparent, ${colors.industrialOrange}40, transparent)`,
          animation: "splash-scan 4s ease-in-out infinite",
        }}
      />

      {/* Radial vignette */}
      <div
        className="absolute inset-0"
        style={{
          background: `radial-gradient(ellipse at center, transparent 0%, ${colors.navyBase}EE 70%, ${colors.navyBase} 100%)`,
        }}
      />

      {/* ── Main card ── */}
      <div
        className="relative z-10 flex flex-col items-center px-8 py-10"
        style={{
          width: cardWidth,
          background: `linear-gradient(180deg, ${colors.navyPanel}F2 0%, ${colors.navyBase}F2 100%)`,
          border: `1px solid ${colors.navyBorder}`,
          borderRadius: "12px",
          boxShadow: `
            0 0 0 1px ${colors.industrialOrange}08,
            0 20px 60px -10px ${colors.navyBase}CC,
            0 0 80px -20px ${colors.industrialOrange}20
          `,
          backdropFilter: "blur(8px)",
        }}
      >
        {/* ── Animated theodolite reticle ── */}
        <TheodoliteReticle size={logoSize} />

        {/* ── Brand name + tagline ── */}
        <div className="mt-6 text-center">
          <h1
            className="font-semibold tracking-[0.15em] text-white"
            style={{ fontSize: isVeryNarrow ? 16 : 18 }}
          >
            {APP_NAME.toUpperCase()}
          </h1>
          <div
            className="mt-1.5 font-mono tracking-[0.25em]"
            style={{
              fontSize: 9,
              color: colors.steelGray,
            }}
          >
            GEODETIC · BATHYMETRIC · INDUSTRIAL
          </div>
        </div>

        {/* ── Divider ── */}
        <div
          className="my-6 h-px w-full"
          style={{
            background: `linear-gradient(90deg, transparent, ${colors.navyBorder}, transparent)`,
          }}
        />

        {/* ── Progress section ── */}
        <div className="w-full">
          {/* Stage label + percentage */}
          <div className="mb-2 flex items-center justify-between">
            <span
              className="font-mono tracking-wider"
              style={{ fontSize: 10, color: colors.steelLight }}
            >
              {STAGES[stageIdx]?.label ?? "Ready"}
            </span>
            <span
              className="font-mono tabular-nums font-semibold"
              style={{ fontSize: 11, color: colors.industrialOrange }}
            >
              {Math.round(progress).toString().padStart(2, "0")}%
            </span>
          </div>

          {/* Progress track */}
          <div
            className="relative h-[3px] w-full overflow-hidden rounded-full"
            style={{ background: colors.navyBorder }}
          >
            <div
              className="absolute inset-y-0 left-0 rounded-full transition-[width] duration-100 ease-out"
              style={{
                width: `${progress}%`,
                background: `linear-gradient(90deg, ${colors.miningYellow} 0%, ${colors.industrialOrange} 50%, ${colors.marineTurquoise} 100%)`,
                boxShadow: `0 0 8px ${colors.industrialOrange}60`,
              }}
            />
          </div>

          {/* Stage indicators */}
          <div className="mt-3 flex justify-between">
            {STAGES.map((_, i) => (
              <div
                key={i}
                className="flex flex-col items-center"
                style={{ flex: 1 }}
              >
                <div
                  className="h-1 w-1 rounded-full transition-colors duration-200"
                  style={{
                    background:
                      i < stageIdx
                        ? colors.pass
                        : i === stageIdx
                          ? colors.industrialOrange
                          : colors.navyBorder,
                    boxShadow:
                      i === stageIdx
                        ? `0 0 6px ${colors.industrialOrange}`
                        : "none",
                  }}
                />
              </div>
            ))}
          </div>
        </div>

        {/* ── Footer: version + build ── */}
        <div
          className="mt-8 flex w-full items-center justify-between font-mono"
          style={{ fontSize: 9, color: colors.steelGray }}
        >
          <span>v{APP_VERSION}</span>
          <span>Build {APP_BUILD}</span>
        </div>
      </div>

      {/* ── Bottom anchor: copyright ── */}
      <div
        className="absolute bottom-5 left-0 right-0 z-10 text-center font-mono"
        style={{ fontSize: 9, color: `${colors.steelGray}80`, letterSpacing: "0.15em" }}
      >
        © 2026 MetaRDU INDUSTRIAL · MIT LICENSE
      </div>

      {/* ── Keyframe animations ── */}
      <style>{`
        @keyframes splash-scan {
          0%, 100% { transform: translateY(0); opacity: 0; }
          10% { opacity: 0.4; }
          50% { transform: translateY(100vh); opacity: 0.4; }
          60% { opacity: 0; }
        }
        @keyframes splash-rotate {
          from { transform: rotate(0deg); }
          to { transform: rotate(360deg); }
        }
        @keyframes splash-rotate-reverse {
          from { transform: rotate(360deg); }
          to { transform: rotate(0deg); }
        }
        @keyframes splash-pulse {
          0%, 100% { opacity: 0.3; transform: scale(1); }
          50% { opacity: 0.8; transform: scale(1.05); }
        }
        @keyframes splash-blink {
          0%, 100% { opacity: 0.2; }
          50% { opacity: 1; }
        }
      `}</style>
    </div>
  );
}

/**
 * Animated SVG theodolite reticle — the survey instrument metaphor.
 *
 * Three concentric rings rotating in alternating directions, with
 * crosshair ticks and a center reticle. Subtle, professional, and
 * clearly survey-related without being cartoonish.
 */
function TheodoliteReticle({ size }: { size: number }) {
  // Use RAW hex for SVG attributes — CSS variables don't work in SVG
  const stroke = rawColors.industrialOrange;
  const strokeDim = `${rawColors.industrialOrange}40`;
  const miningStroke = rawColors.miningYellow;
  const marineStroke = rawColors.marineTurquoise;

  return (
    <div
      style={{ width: size, height: size }}
      className="relative"
    >
      <svg
        width={size}
        height={size}
        viewBox="0 0 200 200"
        fill="none"
        xmlns="http://www.w3.org/2000/svg"
      >
        {/* Outer ring — slow rotation, mining yellow */}
        <g style={{ animation: "splash-rotate 12s linear infinite", transformOrigin: "100px 100px" }}>
          <circle
            cx="100"
            cy="100"
            r="92"
            stroke={miningStroke}
            strokeWidth="1"
            strokeDasharray="2 8"
            opacity="0.5"
          />
          {/* Cardinal ticks (N/E/S/W) */}
          {[0, 90, 180, 270].map((angle) => (
            <line
              key={angle}
              x1="100"
              y1="4"
              x2="100"
              y2="14"
              stroke={miningStroke}
              strokeWidth="2"
              opacity="0.8"
              transform={`rotate(${angle} 100 100)`}
            />
          ))}
        </g>

        {/* Middle ring — faster rotation, industrial orange */}
        <g style={{ animation: "splash-rotate-reverse 8s linear infinite", transformOrigin: "100px 100px" }}>
          <circle
            cx="100"
            cy="100"
            r="75"
            stroke={stroke}
            strokeWidth="1.5"
            strokeDasharray="40 12 8 12"
            opacity="0.7"
          />
          {/* Intercardinal ticks (NE/SE/SW/NW) */}
          {[45, 135, 225, 315].map((angle) => (
            <line
              key={angle}
              x1="100"
              y1="22"
              x2="100"
              y2="30"
              stroke={stroke}
              strokeWidth="1.5"
              opacity="0.6"
              transform={`rotate(${angle} 100 100)`}
            />
          ))}
        </g>

        {/* Inner ring — fastest rotation, marine teal */}
        <g style={{ animation: "splash-rotate 5s linear infinite", transformOrigin: "100px 100px" }}>
          <circle
            cx="100"
            cy="100"
            r="55"
            stroke={marineStroke}
            strokeWidth="1"
            strokeDasharray="3 6"
            opacity="0.6"
          />
        </g>

        {/* Crosshair — static */}
        <line x1="100" y1="40" x2="100" y2="160" stroke={strokeDim} strokeWidth="1" />
        <line x1="40" y1="100" x2="160" y2="100" stroke={strokeDim} strokeWidth="1" />

        {/* Center reticle — pulsing */}
        <g style={{ animation: "splash-pulse 2s ease-in-out infinite", transformOrigin: "100px 100px" }}>
          <circle cx="100" cy="100" r="20" stroke={stroke} strokeWidth="1.5" fill="none" opacity="0.5" />
          <circle cx="100" cy="100" r="12" stroke={stroke} strokeWidth="1" fill="none" opacity="0.8" />
          <circle cx="100" cy="100" r="3" fill={stroke} />
        </g>

        {/* Blinking status LED at top */}
        <circle
          cx="100"
          cy="8"
          r="2"
          fill={rawColors.pass}
          style={{ animation: "splash-blink 1.5s ease-in-out infinite" }}
        />
      </svg>
    </div>
  );
}
