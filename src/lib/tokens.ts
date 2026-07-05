/**
 * MetaRDU Industrial — Design Tokens
 * Single source of truth for brand colors, extracted from the logo.
 * Used by Tailwind via @theme in index.css and directly in components.
 *
 * IMPORTANT: Colors use CSS variable references (var(--color-*)) so
 * they respect the theme override in index.css (:root[data-theme="light"]).
 * This means inline styles like `style={{ background: colors.navyBase }}`
 * will automatically switch to the light-theme values when the user
 * toggles to daylight mode. Previously these were hardcoded hex values
 * which ignored the theme override.
 */

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

export const APP_VERSION = "0.1.0";
export const APP_BUILD = "2026.07.02";
export const APP_NAME = "MetaRDU Industrial";
export const APP_TAGLINE = "Mining & Marine Surveys";
