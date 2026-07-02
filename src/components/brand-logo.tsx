/**
 * MetaRDU Industrial — Brand Logo
 * SVG recreation of the MetaRDU Industrial logo.
 * Composed of:
 *   - Theodolite / total station (central focal point, orange)
 *   - Split lens (mining terrain top, marine water bottom)
 *   - Coordinate grid overlay in lens
 *   - "M" frame in background (white)
 *   - Wordmark below
 *
 * Used on splash, onboarding, title bar, About dialog.
 */

import { colors } from "@/lib/tokens";

interface BrandLogoProps {
  size?: number;
  showWordmark?: boolean;
  animated?: boolean;
  className?: string;
}

export function BrandLogo({
  size = 200,
  showWordmark = true,
  animated = false,
  className = "",
}: BrandLogoProps) {
  const id = "metardu-logo";
  return (
    <div className={`flex flex-col items-center ${className}`}>
      <svg
        width={size}
        height={size}
        viewBox="0 0 240 240"
        fill="none"
        xmlns="http://www.w3.org/2000/svg"
        role="img"
        aria-label="MetaRDU Industrial logo"
      >
        <defs>
          <clipPath id={`${id}-lens-clip`}>
            <circle cx="120" cy="120" r="44" />
          </clipPath>
          <linearGradient id={`${id}-mining-grad`} x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor={colors.miningBurnt} />
            <stop offset="100%" stopColor={colors.miningTerrain} />
          </linearGradient>
          <linearGradient id={`${id}-marine-grad`} x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor={colors.marineTurquoise} />
            <stop offset="100%" stopColor={colors.marineDeep} />
          </linearGradient>
        </defs>

        {/* Background M frame (white, semi-transparent) */}
        <path
          d="M40 200 L40 60 L80 60 L120 130 L160 60 L200 60 L200 200 L170 200 L170 110 L130 180 L110 180 L70 110 L70 200 Z"
          stroke={colors.white}
          strokeWidth="3"
          fill="none"
          opacity="0.18"
          strokeLinejoin="round"
        />

        {/* Theodolite frame — vertical uprights + crossbar */}
        <g
          className={animated ? "animate-lens-rotate" : ""}
          style={{ transformOrigin: "120px 120px" }}
        >
          {/* Outer ring */}
          <circle
            cx="120"
            cy="120"
            r="52"
            stroke={colors.industrialOrange}
            strokeWidth="3"
            fill="none"
          />
          <circle
            cx="120"
            cy="120"
            r="48"
            stroke={colors.industrialOrange}
            strokeWidth="1"
            fill="none"
            opacity="0.5"
          />

          {/* Split lens — mining top half */}
          <g clipPath={`url(#${id}-lens-clip)`}>
            <rect
              x="76"
              y="76"
              width="88"
              height="44"
              fill={`url(#${id}-mining-grad)`}
            />
            <rect
              x="76"
              y="120"
              width="88"
              height="44"
              fill={`url(#${id}-marine-grad)`}
            />

            {/* Coordinate grid overlay */}
            <g stroke={colors.white} strokeWidth="0.5" opacity="0.4">
              {[88, 100, 112, 124, 136, 148].map((x) => (
                <line key={`v-${x}`} x1={x} y1="76" x2={x} y2="164" />
              ))}
              {[88, 100, 112, 124, 136, 148].map((y) => (
                <line key={`h-${y}`} x1="76" y1={y} x2="164" y2={y} />
              ))}
            </g>

            {/* Mining terrain silhouette */}
            <path
              d="M76 120 L88 110 L96 116 L108 104 L120 112 L132 102 L144 110 L156 104 L164 108 L164 120 Z"
              fill={colors.miningTerrain}
              opacity="0.85"
            />

            {/* Marine wave line */}
            <path
              d="M76 130 Q88 126 100 130 T124 130 T148 130 T164 130"
              stroke={colors.marineCyan}
              strokeWidth="1.2"
              fill="none"
              opacity="0.7"
            />

            {/* Crosshair center */}
            <line
              x1="120"
              y1="100"
              x2="120"
              y2="140"
              stroke={colors.industrialOrange}
              strokeWidth="0.8"
            />
            <line
              x1="100"
              y1="120"
              x2="140"
              y2="120"
              stroke={colors.industrialOrange}
              strokeWidth="0.8"
            />
            <circle cx="120" cy="120" r="2" fill={colors.industrialOrange} />
          </g>

          {/* Theodolite knobs (side handles) */}
          <rect
            x="64"
            y="116"
            width="10"
            height="8"
            rx="2"
            fill={colors.industrialOrange}
          />
          <rect
            x="166"
            y="116"
            width="10"
            height="8"
            rx="2"
            fill={colors.industrialOrange}
          />

          {/* Top handle */}
          <rect
            x="116"
            y="56"
            width="8"
            height="14"
            rx="2"
            fill={colors.industrialOrange}
          />
          <circle cx="120" cy="54" r="4" fill={colors.industrialOrange} />

          {/* Tripod base */}
          <rect
            x="116"
            y="172"
            width="8"
            height="10"
            fill={colors.industrialOrange}
          />
          <line
            x1="120"
            y1="182"
            x2="100"
            y2="208"
            stroke={colors.industrialOrange}
            strokeWidth="2.5"
            strokeLinecap="round"
          />
          <line
            x1="120"
            y1="182"
            x2="140"
            y2="208"
            stroke={colors.industrialOrange}
            strokeWidth="2.5"
            strokeLinecap="round"
          />
          <line
            x1="120"
            y1="182"
            x2="120"
            y2="212"
            stroke={colors.industrialOrange}
            strokeWidth="2.5"
            strokeLinecap="round"
          />
        </g>

        {/* Corner ticks — survey corner marks */}
        <g stroke={colors.industrialOrange} strokeWidth="1.5" opacity="0.6">
          <path d="M16 16 L16 28 M16 16 L28 16" />
          <path d="M224 16 L224 28 M224 16 L212 16" />
          <path d="M16 224 L16 212 M16 224 L28 224" />
          <path d="M224 224 L224 212 M224 224 L212 224" />
        </g>
      </svg>

      {showWordmark && (
        <div className="mt-4 text-center">
          <div className="text-2xl font-bold tracking-wider">
            <span className="text-white">META</span>
            <span style={{ color: colors.industrialOrange }}>RDU</span>
          </div>
          <div className="mt-1 flex items-center justify-center gap-2 text-xs tracking-[0.3em] text-steel-gray">
            <span className="h-px w-6 bg-steel-gray" />
            <span>INDUSTRIAL</span>
            <span className="h-px w-6 bg-steel-gray" />
          </div>
          <div
            className="mt-1 text-[10px] tracking-[0.25em] font-medium"
            style={{ color: colors.industrialOrange }}
          >
            MINING &amp; MARINE SURVEYS
          </div>
        </div>
      )}
    </div>
  );
}
