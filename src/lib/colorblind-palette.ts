/**
 * Colorblind-safe palette — Sprint 17.
 *
 * 8% of male surveyors have some form of color vision deficiency.
 * The default MetaRDU palette uses red (cut) and green (fill) which
 * is indistinguishable for red-green colorblindness (protanopia,
 * deuteranopia). This module provides a colorblind-safe palette that
 * swaps:
 *   - Red → Orange (still warm, but distinguishable from green)
 *   - Green → Blue (cool, clearly different from orange)
 *   - Status colors use the Wong (2011) palette
 *
 * The toggle is persisted in localStorage and applies a `data-palette`
 * attribute to <html> so CSS can override colors.
 *
 * Reference: Wong, B. (2011) "Color blindness." Nature Methods 8:441.
 */

import { useEffect, useState, useCallback } from "react";

export type PaletteMode = "default" | "colorblind";

const STORAGE_KEY = "metardu-palette-mode";

export function getPaletteMode(): PaletteMode {
  if (typeof window === "undefined") return "default";
  return (localStorage.getItem(STORAGE_KEY) as PaletteMode) || "default";
}

export function setPaletteMode(mode: PaletteMode) {
  localStorage.setItem(STORAGE_KEY, mode);
  document.documentElement.setAttribute("data-palette", mode);
  window.dispatchEvent(new CustomEvent("metardu-palette-change", { detail: mode }));
}

/**
 * Hook that returns the current palette mode + a setter.
 * Applies the `data-palette` attribute to <html> on change.
 */
export function useColorblindPalette() {
  const [mode, setMode] = useState<PaletteMode>(getPaletteMode);

  useEffect(() => {
    document.documentElement.setAttribute("data-palette", mode);
    const onChange = (e: Event) => {
      const detail = (e as CustomEvent).detail as PaletteMode;
      setMode(detail);
    };
    window.addEventListener("metardu-palette-change", onChange);
    return () => window.removeEventListener("metardu-palette-change", onChange);
  }, []);

  const toggle = useCallback(() => {
    const newMode: PaletteMode = mode === "default" ? "colorblind" : "default";
    setPaletteMode(newMode);
    setMode(newMode);
  }, [mode]);

  return { mode, toggle, setMode: (m: PaletteMode) => { setPaletteMode(m); setMode(m); } };
}

/**
 * Get a color value, respecting the current palette mode.
 * Usage: `const cutColor = getColor("#EF4444", "#FB923C"); // red or orange`
 */
export function getColor(defaultColor: string, colorblindColor: string): string {
  return getPaletteMode() === "colorblind" ? colorblindColor : defaultColor;
}

/**
 * Standard color remapping for the colorblind palette.
 * Keyed by semantic name so callers can use `COLORBLIND_MAP.cut` etc.
 */
export const COLORBLIND_MAP = {
  // Cut/fill — the most important colorblind-safe swap
  cut: { default: "#EF4444", colorblind: "#E69F00" },        // red → orange
  fill: { default: "#22C55E", colorblind: "#56B4E9" },       // green → sky blue
  // Status semantics
  pass: { default: "#22C55E", colorblind: "#56B4E9" },       // green → sky blue
  fail: { default: "#EF4444", colorblind: "#D55E00" },       // red → vermillion
  warn: { default: "#F59E0B", colorblind: "#E69F00" },       // amber → orange (stays warm)
  info: { default: "#3B82F6", colorblind: "#0072B2" },       // blue → darker blue
  // Domain accents
  mining: { default: "#FBBF24", colorblind: "#E69F00" },     // amber → orange
  marine: { default: "#2DD4BF", colorblind: "#009E73" },     // teal → bluish green
} as const;

export type ColorblindKey = keyof typeof COLORBLIND_MAP;

/** Get a semantic color respecting the current palette mode. */
export function getSemanticColor(key: ColorblindKey): string {
  const entry = COLORBLIND_MAP[key];
  return getColor(entry.default, entry.colorblind);
}
