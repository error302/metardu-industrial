/**
 * MetaRDU Industrial — Design Tokens
 * Single source of truth for brand colors, extracted from the logo.
 *
 * IMPORTANT: Two sets of colors are provided:
 *   - `colors` — CSS variable references (var(--color-*)) for HTML
 *     inline styles. These respect the theme override in index.css.
 *   - `rawColors` — raw hex values for Canvas/SVG contexts (OpenLayers,
 *     Deck.gl, SVG attributes) where CSS variables don't work.
 *
 * Use `colors` for React inline styles (style={{ background: colors.navyBase }}).
 * Use `rawColors` for OpenLayers/Canvas/SVG (new Stroke({ color: rawColors.miningYellow })).
 */

/** Raw hex values — for Canvas/SVG/OpenLayers/Deck.gl contexts. */
export const rawColors = {
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

/** CSS variable references — for React inline styles. These respect
 * the light/dark theme override in index.css. */
export const colors = {
  navyBase: "var(--color-navy-base)",
  navyPanel: "var(--color-navy-panel)",
  navyElevated: "var(--color-navy-elevated)",
  navyBorder: "var(--color-navy-border)",

  industrialOrange: "var(--color-industrial-orange)",
  industrialOrangeDim: "var(--color-industrial-orange-dim)",

  white: "var(--color-white)",
  steelGray: "var(--color-steel-gray)",
  steelLight: "var(--color-steel-light)",

  // Mining mode
  miningYellow: "var(--color-mining-yellow)",
  miningBurnt: "var(--color-mining-burnt)",
  miningTerrain: "var(--color-mining-terrain)",

  // Marine mode
  marineDeep: "var(--color-marine-deep)",
  marineTurquoise: "var(--color-marine-turquoise)",
  marineCyan: "var(--color-marine-cyan)",

  // Semantic
  pass: "var(--color-pass)",
  investigate: "var(--color-investigate)",
  fail: "var(--color-fail)",
  info: "var(--color-info)",
} as const;

export type DomainMode = "mining" | "marine" | "both";

/** CSS-variable-based accent (for HTML inline styles). */
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

/** Raw-hex-based accent (for Canvas/SVG/OpenLayers). */
export const rawDomainAccent: Record<
  DomainMode,
  { primary: string; secondary: string; label: string }
> = {
  mining: {
    primary: rawColors.miningYellow,
    secondary: rawColors.miningBurnt,
    label: "Mining",
  },
  marine: {
    primary: rawColors.marineTurquoise,
    secondary: rawColors.marineCyan,
    label: "Marine",
  },
  both: {
    primary: rawColors.industrialOrange,
    secondary: rawColors.miningBurnt,
    label: "Mining & Marine",
  },
};

export const APP_VERSION = "0.1.0";
export const APP_BUILD = "2026.07.02";
export const APP_NAME = "MetaRDU Industrial";
export const APP_TAGLINE = "Mining & Marine Surveys";
