/**
 * MetaRDU Industrial — Official Brand Logo
 *
 * Professional survey-target logo mark designed to read clearly at
 * every size from 16px (favicon) to 240px (splash screen).
 *
 * Design elements:
 *   - Outer orange ring (survey target / theodolite lens)
 *   - Inner crosshair (survey monument marker)
 *   - Cardinal direction ticks (N/E/S/W)
 *   - Bold "M" monogram at center
 *   - Optional wordmark: "MetaRDU" + "INDUSTRIAL" + tagline
 *
 * The ring rotates on the splash screen (animated=true) while the
 * crosshair + "M" stay fixed — a survey-target "spinning up" metaphor.
 *
 * Color palette (from official brand):
 *   - Industrial Orange: #FFA500 (primary accent)
 *   - Gold: #FFC107 (secondary accent, cardinal ticks)
 *   - Navy: #0A192F (background)
 *   - White: #FFFFFF (text/crosshair)
 *   - Steel: #64748B (secondary text)
 */

import { colors, rawColors } from "@/lib/tokens";

interface BrandLogoProps {
  size?: number;
  showWordmark?: boolean;
  animated?: boolean;
  className?: string;
}

export function BrandLogo({
  size = 240,
  showWordmark = true,
  animated = false,
  className = "",
}: BrandLogoProps) {
  // Use RAW hex for SVG attributes — CSS variables don't work in SVG fill/stroke
  const orange = rawColors.industrialOrange;
  const gold = rawColors.miningYellow;
  const navy = rawColors.navyBase;
  const white = rawColors.white;
  const steel = rawColors.steelGray;

  return (
    <div className={`flex flex-col items-center ${className}`}>
      <svg
        width={size}
        height={size}
        viewBox="0 0 200 200"
        fill="none"
        xmlns="http://www.w3.org/2000/svg"
        role="img"
        aria-label="MetaRDU Industrial logo"
      >
        {/* ── Background circle (navy fill for contrast) ── */}
        <circle cx="100" cy="100" r="96" fill={navy} />

        {/* ── Rotating outer ring + cardinal ticks (animated on splash) ── */}
        <g
          className={animated ? "animate-lens-rotate" : ""}
          style={{ transformOrigin: "100px 100px" }}
        >
          {/* Outer ring — survey target / theodolite lens */}
          <circle
            cx="100"
            cy="100"
            r="90"
            stroke={orange}
            strokeWidth="4"
            fill="none"
          />

          {/* Inner reference ring — finer, lower opacity */}
          <circle
            cx="100"
            cy="100"
            r="72"
            stroke={orange}
            strokeWidth="1"
            fill="none"
            opacity="0.3"
          />

          {/* Cardinal tick marks — N/E/S/W (gold for emphasis) */}
          {/* North tick — bolder, with arrow indicator */}
          <line x1="100" y1="4" x2="100" y2="20" stroke={gold} strokeWidth="3" strokeLinecap="round" />
          <polygon points="100,2 96,12 104,12" fill={gold} />

          {/* South tick */}
          <line x1="100" y1="180" x2="100" y2="196" stroke={orange} strokeWidth="2.5" strokeLinecap="round" />

          {/* East tick */}
          <line x1="180" y1="100" x2="196" y2="100" stroke={orange} strokeWidth="2.5" strokeLinecap="round" />

          {/* West tick */}
          <line x1="4" y1="100" x2="20" y2="100" stroke={orange} strokeWidth="2.5" strokeLinecap="round" />

          {/* Intermediate ticks (NE/SE/SW/NW) — smaller, steel color */}
          {[45, 135, 225, 315].map((angle) => {
            const rad = (angle * Math.PI) / 180;
            const x1 = 100 + 88 * Math.cos(rad);
            const y1 = 100 + 88 * Math.sin(rad);
            const x2 = 100 + 96 * Math.cos(rad);
            const y2 = 100 + 96 * Math.sin(rad);
            return (
              <line
                key={angle}
                x1={x1}
                y1={y1}
                x2={x2}
                y2={y2}
                stroke={steel}
                strokeWidth="1.5"
                strokeLinecap="round"
              />
            );
          })}
        </g>

        {/* ── Inner crosshair (survey monument marker) — fixed, non-rotating ── */}
        <g opacity="0.15">
          {/* Horizontal crosshair line */}
          <line x1="40" y1="100" x2="160" y2="100" stroke={white} strokeWidth="0.5" />
          {/* Vertical crosshair line */}
          <line x1="100" y1="40" x2="100" y2="160" stroke={white} strokeWidth="0.5" />
        </g>

        {/* ── Center "M" monogram ── */}
        {/* Background circle for the M — dark navy, slightly lighter than outer */}
        <circle cx="100" cy="100" r="48" fill={navy} stroke={orange} strokeWidth="1.5" opacity="0.95" />

        {/* The M lettermark — bold, white, centered */}
        <text
          x="100"
          y="118"
          textAnchor="middle"
          fontSize="56"
          fontWeight="900"
          fontFamily="Inter, system-ui, -apple-system, sans-serif"
          fill={white}
          style={{ letterSpacing: "-2px" }}
        >
          M
        </text>

        {/* ── Small accent dots at NE/SE/SW/NW of inner circle ── */}
        {[45, 135, 225, 315].map((angle) => {
          const rad = (angle * Math.PI) / 180;
          const x = 100 + 48 * Math.cos(rad);
          const y = 100 + 48 * Math.sin(rad);
          return (
            <circle key={`dot-${angle}`} cx={x} cy={y} r="1.5" fill={gold} opacity="0.8" />
          );
        })}
      </svg>

      {showWordmark && (
        <div className="mt-4 text-center">
          {/* Main wordmark: MetaRDU */}
          <div
            className="text-3xl font-extrabold tracking-wider leading-none"
            style={{ fontFamily: "Inter, system-ui, -apple-system, sans-serif" }}
          >
            <span className="text-white">Meta</span>
            <span style={{ color: orange }}>RDU</span>
          </div>

          {/* Divider + subtitle */}
          <div className="mt-2 flex items-center justify-center gap-3">
            <span className="h-px w-8" style={{ background: steel }} />
            <span
              className="text-xs font-bold tracking-[0.35em] text-steel-light"
              style={{ fontFamily: "Inter, system-ui, sans-serif" }}
            >
              INDUSTRIAL
            </span>
            <span className="h-px w-8" style={{ background: steel }} />
          </div>

          {/* Tagline */}
          <div
            className="mt-1.5 text-[10px] font-semibold tracking-[0.2em]"
            style={{ color: orange, fontFamily: "Inter, system-ui, sans-serif" }}
          >
            MINING &amp; MARINE SURVEYS
          </div>
        </div>
      )}
    </div>
  );
}

/**
 * Compact logo — just the mark, no wordmark. For title bars, sidebars,
 * and places where space is tight.
 */
export function BrandLogoMark({
  size = 32,
  className = "",
}: {
  size?: number;
  className?: string;
}) {
  const orange = colors.industrialOrange;
  const gold = "#FFC107";
  const navy = colors.navyBase;
  const white = colors.white;

  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 200 200"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      role="img"
      aria-label="MetaRDU Industrial"
      className={className}
    >
      {/* Background */}
      <circle cx="100" cy="100" r="96" fill={navy} />

      {/* Outer ring */}
      <circle cx="100" cy="100" r="90" stroke={orange} strokeWidth="4" fill="none" />

      {/* N tick with arrow (gold) */}
      <line x1="100" y1="4" x2="100" y2="20" stroke={gold} strokeWidth="3" strokeLinecap="round" />
      <polygon points="100,2 96,12 104,12" fill={gold} />

      {/* S/E/W ticks */}
      <line x1="100" y1="180" x2="100" y2="196" stroke={orange} strokeWidth="2.5" strokeLinecap="round" />
      <line x1="180" y1="100" x2="196" y2="100" stroke={orange} strokeWidth="2.5" strokeLinecap="round" />
      <line x1="4" y1="100" x2="20" y2="100" stroke={orange} strokeWidth="2.5" strokeLinecap="round" />

      {/* Inner circle */}
      <circle cx="100" cy="100" r="48" fill={navy} stroke={orange} strokeWidth="1.5" />

      {/* M monogram */}
      <text
        x="100"
        y="118"
        textAnchor="middle"
        fontSize="56"
        fontWeight="900"
        fontFamily="Inter, system-ui, -apple-system, sans-serif"
        fill={white}
        style={{ letterSpacing: "-2px" }}
      >
        M
      </text>
    </svg>
  );
}
