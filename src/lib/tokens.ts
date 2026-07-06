/**
 * MetaRDU Industrial — Design Tokens
 *
 * Professional GIS color system based on QGIS/CloudCompare dark themes.
 * Neutral slate chrome so map data is the only saturated color.
 *
 * Principles:
 *   - Chrome uses a slate ramp (darker = lower elevation in UI hierarchy)
 *   - One accent color (industrial orange) for active/selected states
 *   - Status colors are reserved for semantics (red/amber/green/blue)
 *   - Map canvas background is dark neutral
 */

export const colors = {
  // ── Slate chrome ramp (dark → light) ──
  // Use darker shades for lower UI layers (status bar), lighter for raised (panels)
  base: "#0F172A",        // slate-900 — app background, map canvas bg
  panel: "#1E293B",       // slate-800 — sidebars, panels, dialogs
  elevated: "#334155",    // slate-700 — raised elements, hover states
  border: "#475569",      // slate-600 — borders, dividers
  borderLight: "#64748B", // slate-500 — lighter borders

  // ── Text ──
  white: "#F1F5F9",       // slate-100 — primary text (not pure white, easier on eyes)
  textSecondary: "#94A3B8", // slate-400 — secondary text
  textMuted: "#64748B",   // slate-500 — muted/placeholder text

  // ── Accent ──
  accent: "#F97316",      // orange-500 — active/selected/primary action
  accentDim: "#C2410C",   // orange-700 — hover/pressed
  accentLight: "#FB923C", // orange-400 — focus rings

  // ── Domain colors (for mining/marine accents) ──
  mining: "#FBBF24",      // amber-400
  miningDim: "#D97706",   // amber-600
  marine: "#2DD4BF",      // teal-400
  marineDim: "#0F766E",   // teal-700

  // ── Status semantics ──
  pass: "#22C55E",        // green-500
  passDim: "#15803D",
  warn: "#F59E0B",        // amber-500
  fail: "#EF4444",        // red-500
  failDim: "#B91C1C",
  info: "#3B82F6",        // blue-500

  // ── Aliases for backward compatibility ──
  navyBase: "#0F172A",
  navyPanel: "#1E293B",
  navyElevated: "#334155",
  navyBorder: "#475569",
  industrialOrange: "#F97316",
  industrialOrangeDim: "#C2410C",
  steelGray: "#64748B",
  steelLight: "#94A3B8",
  miningYellow: "#FBBF24",
  miningBurnt: "#D97706",
  miningTerrain: "#78350F",
  marineDeep: "#1E40AF",
  marineTurquoise: "#2DD4BF",
  marineCyan: "#06B6D4",
  investigate: "#F59E0B",
} as const;

/** Alias for rawColors — same values. */
export const rawColors = colors;

export type DomainMode = "mining" | "marine" | "both";

export const domainAccent: Record<
  DomainMode,
  { primary: string; secondary: string; label: string }
> = {
  mining: {
    primary: colors.mining,
    secondary: colors.miningDim,
    label: "Mining",
  },
  marine: {
    primary: colors.marine,
    secondary: colors.marineDim,
    label: "Marine",
  },
  both: {
    primary: colors.accent,
    secondary: colors.miningDim,
    label: "Mining & Marine",
  },
};

export const rawDomainAccent = domainAccent;

export const APP_VERSION = "0.1.0";
export const APP_BUILD = "2026.07.02";
export const APP_NAME = "MetaRDU Industrial";
export const APP_TAGLINE = "Mining & Marine Surveys";
