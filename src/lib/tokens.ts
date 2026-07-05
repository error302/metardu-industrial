/**
 * MetaRDU Industrial — Design Tokens
 * Single source of truth for brand colors, extracted from the logo.
 *
 * Colors are raw hex values. Theme switching is handled via CSS
 * classes (data-theme attribute + CSS variable overrides in index.css).
 * Components that need theme-aware colors should use Tailwind classes
 * (bg-navy-base, text-white, etc.) which respect the CSS overrides.
 *
 * Inline styles using `colors.*` use raw hex — these do NOT change
 * with the theme. This is intentional: Canvas/SVG/OL contexts need
 * raw hex, and the `${colors.industrialOrange}40` alpha-append
 * pattern only works with raw hex, not CSS variables.
 */

export const colors = {
  navyBase: "#0A192F",
  navyPanel: "#0F1F3A",
  navyElevated: "#142A4A",
  navyBorder: "#1E2A3F",

  industrialOrange: "#FFA500",
  industrialOrangeDim: "#B8771A",

  white: "#FFFFFF",
  steelGray: "#6B7280",
  steelLight: "#9CA3AF",

  // Mining mode
  miningYellow: "#FFC107",
  miningBurnt: "#FFB347",
  miningTerrain: "#8B4513",

  // Marine mode
  marineDeep: "#1E3A8A",
  marineTurquoise: "#20B2AA",
  marineCyan: "#06B6D4",

  // Semantic
  pass: "#10B981",
  investigate: "#F59E0B",
  fail: "#EF4444",
  info: "#3B82F6",
} as const;

/** Alias for rawColors — same values. Kept for backward compat
 * with components that already import rawColors. */
export const rawColors = colors;

export type DomainMode = "mining" | "marine" | "both";

export const domainAccent: Record<
  DomainMode,
  { primary: string; secondary: string; label: string }
> = {
  mining: {
    primary: colors.miningYellow,
    secondary: colors.miningBurnt,
    label: "Mining",
  },
  marine: {
    primary: colors.marineTurquoise,
    secondary: colors.marineCyan,
    label: "Marine",
  },
  both: {
    primary: colors.industrialOrange,
    secondary: colors.miningBurnt,
    label: "Mining & Marine",
  },
};

/** Alias — same as domainAccent. Kept for backward compat. */
export const rawDomainAccent = domainAccent;

export const APP_VERSION = "0.1.0";
export const APP_BUILD = "2026.07.02";
export const APP_NAME = "MetaRDU Industrial";
export const APP_TAGLINE = "Mining & Marine Surveys";
