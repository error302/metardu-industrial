/**
 * Viewport size hook — drives responsive layout decisions across the app.
 *
 * The workspace shell needs to adapt at multiple breakpoints:
 *   - Below `lg` (1024px): right panel auto-collapses, map gets priority
 *   - Below `md` (768px): left sidebar switches to icon-only rail
 *   - Below `sm` (640px): sidebars become overlay drawers
 *
 * This hook returns discrete flags so components can branch in JS rather
 * than pile up Tailwind responsive variants. SSR is not a concern — this
 * is a Tauri desktop app.
 */

import { useEffect, useState } from "react";

export interface ViewportSize {
  width: number;
  height: number;
  /** < 640px — phone-class width. Sidebars become drawers. */
  isVeryNarrow: boolean;
  /** < 768px — tablet-class width. Sidebar switches to icon-only rail. */
  isNarrow: boolean;
  /** < 1024px — small laptop. Right panel auto-collapses. */
  isCompact: boolean;
  /** < 1280px — typical laptop. */
  isMedium: boolean;
  /** ≥ 1280px — desktop. */
  isWide: boolean;
}

const BREAKPOINTS = {
  sm: 640,
  md: 768,
  lg: 1024,
  xl: 1280,
} as const;

function computeSize(): ViewportSize {
  // Guard for non-browser environments (Tauri main process, tests)
  if (typeof window === "undefined") {
    return {
      width: 1920,
      height: 1080,
      isVeryNarrow: false,
      isNarrow: false,
      isCompact: false,
      isMedium: false,
      isWide: true,
    };
  }
  const width = window.innerWidth;
  const height = window.innerHeight;
  return {
    width,
    height,
    isVeryNarrow: width < BREAKPOINTS.sm,
    isNarrow: width < BREAKPOINTS.md,
    isCompact: width < BREAKPOINTS.lg,
    isMedium: width >= BREAKPOINTS.lg && width < BREAKPOINTS.xl,
    isWide: width >= BREAKPOINTS.xl,
  };
}

export function useViewport(): ViewportSize {
  const [size, setSize] = useState<ViewportSize>(computeSize);

  useEffect(() => {
    let frame = 0;
    const update = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => setSize(computeSize()));
    };
    update();
    window.addEventListener("resize", update);
    window.addEventListener("orientationchange", update);
    return () => {
      cancelAnimationFrame(frame);
      window.removeEventListener("resize", update);
      window.removeEventListener("orientationchange", update);
    };
  }, []);

  return size;
}
