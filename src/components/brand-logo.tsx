/**
 * MetaRDU Industrial — Brand Logo
 *
 * Simplified, bolder mark designed to read clearly at every size:
 *   - 32px  → favicon / title bar (showWordmark=false)
 *   - 64px  → toolbar glyph
 *   - 240px → splash screen
 *
 * Composition:
 *   - Large orange ring (survey target / theodolite lens)
 *   - Cardinal tick marks at N/S/E/W
 *   - Bold white "M" lettermark at the center
 *   - "METARDU" wordmark below — "META" white, "RDU" orange
 *   - Tagline: "MINING & MARINE SURVEYS"
 *
 * The ring + ticks rotate on the splash screen (animated=true) while
 * the central "M" stays put — a survey-target "spinning up" metaphor.
 */

import { colors } from "@/lib/tokens";

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
  const orange = colors.industrialOrange;

  return (
    <div className={`flex flex-col items-center ${className}`}>
      <svg
        width={size}
        height={size}
        viewBox="0 0 120 120"
        fill="none"
        xmlns="http://www.w3.org/2000/svg"
        role="img"
        aria-label="MetaRDU Industrial logo"
      >
        {/* Rotating ring + cardinal ticks (animated on splash) */}
        <g
          className={animated ? "animate-lens-rotate" : ""}
          style={{ transformOrigin: "60px 60px" }}
        >
          {/* Outer ring — survey target / theodolite lens */}
          <circle
            cx="60"
            cy="60"
            r="54"
            stroke={orange}
            strokeWidth="6"
            fill="none"
          />
          {/* Inner reference ring — finer, lower opacity */}
          <circle
            cx="60"
            cy="60"
            r="44"
            stroke={orange}
            strokeWidth="1.5"
            fill="none"
            opacity="0.35"
          />
          {/* Cardinal tick marks (N/S/E/W) */}
          <line
            x1="60"
            y1="2"
            x2="60"
            y2="14"
            stroke={orange}
            strokeWidth="3.5"
            strokeLinecap="round"
          />
          <line
            x1="60"
            y1="106"
            x2="60"
            y2="118"
            stroke={orange}
            strokeWidth="3.5"
            strokeLinecap="round"
          />
          <line
            x1="2"
            y1="60"
            x2="14"
            y2="60"
            stroke={orange}
            strokeWidth="3.5"
            strokeLinecap="round"
          />
          <line
            x1="106"
            y1="60"
            x2="118"
            y2="60"
            stroke={orange}
            strokeWidth="3.5"
            strokeLinecap="round"
          />
        </g>

        {/* Bold "M" lettermark — centered, white for maximum contrast */}
        <text
          x="60"
          y="82"
          textAnchor="middle"
          fontSize="62"
          fontWeight="900"
          fontFamily="Inter, system-ui, -apple-system, sans-serif"
          fill={colors.white}
        >
          M
        </text>
      </svg>

      {showWordmark && (
        <div className="mt-5 text-center">
          <div className="text-3xl font-extrabold tracking-wider leading-none">
            <span className="text-white">META</span>
            <span style={{ color: orange }}>RDU</span>
          </div>
          <div className="mt-2 flex items-center justify-center gap-2 text-xs tracking-[0.3em] text-steel-light">
            <span className="h-px w-6 bg-steel-gray" />
            <span>INDUSTRIAL</span>
            <span className="h-px w-6 bg-steel-gray" />
          </div>
          <div
            className="mt-1.5 text-[11px] tracking-[0.25em] font-semibold"
            style={{ color: orange }}
          >
            MINING &amp; MARINE SURVEYS
          </div>
        </div>
      )}
    </div>
  );
}
