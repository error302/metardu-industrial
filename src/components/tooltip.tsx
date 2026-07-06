/**
 * Tooltip — accessible hover tooltip (Sprint 13).
 *
 * Native `title=` attributes are too slow (1-2 second delay) and can't
 * be styled. This component shows a tooltip instantly on hover with
 * consistent dark styling.
 *
 * Usage:
 * ```tsx
 * <Tooltip text="Compute volume (Ctrl+Enter)">
 *   <button>...</button>
 * </Tooltip>
 * ```
 *
 * Accessibility: the tooltip text is also set as `aria-label` on the
 * wrapper so screen readers announce it.
 */

import { useState, useRef, type ReactNode } from "react";
import { colors } from "@/lib/tokens";

interface TooltipProps {
  /** Tooltip text. */
  text: string;
  /** Where to position the tooltip relative to the trigger. Default: "top". */
  position?: "top" | "bottom" | "left" | "right";
  /** Delay before showing, in ms. Default: 300. */
  delay?: number;
  /** The trigger element. */
  children: ReactNode;
}

export function Tooltip({ text, position = "top", delay = 300, children }: TooltipProps) {
  const [visible, setVisible] = useState(false);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  function show() {
    if (timerRef.current) clearTimeout(timerRef.current);
    timerRef.current = setTimeout(() => setVisible(true), delay);
  }
  function hide() {
    if (timerRef.current) clearTimeout(timerRef.current);
    setVisible(false);
  }

  const positionClasses: Record<string, string> = {
    top: "bottom-full left-1/2 -translate-x-1/2 mb-1.5",
    bottom: "top-full left-1/2 -translate-x-1/2 mt-1.5",
    left: "right-full top-1/2 -translate-y-1/2 mr-1.5",
    right: "left-full top-1/2 -translate-y-1/2 ml-1.5",
  };

  return (
    <span
      className="relative inline-flex"
      onMouseEnter={show}
      onMouseLeave={hide}
      onFocus={show}
      onBlur={hide}
      aria-label={text}
    >
      {children}
      {visible && (
        <span
          role="tooltip"
          className={`absolute z-[100] pointer-events-none whitespace-nowrap rounded px-2 py-1 text-[10px] font-medium shadow-lg ${positionClasses[position]}`}
          style={{
            background: colors.base,
            color: colors.white,
            border: `1px solid ${colors.border}`,
            maxWidth: "240px",
          }}
        >
          {text}
        </span>
      )}
    </span>
  );
}
