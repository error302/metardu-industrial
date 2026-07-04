/**
 * useEscapeKey — calls a callback when the Escape key is pressed.
 *
 * Used by all custom modal dialogs so the user can close them with Esc
 * without hunting for the X button.
 *
 * Usage:
 *   useEscapeKey(onClose);
 *   useEscapeKey(onClose, open); // only active when `open` is true
 */

import { useEffect } from "react";

export function useEscapeKey(callback: () => void, enabled: boolean = true) {
  useEffect(() => {
    if (!enabled) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        callback();
      }
    };
    window.addEventListener("keydown", handler, true);
    return () => window.removeEventListener("keydown", handler, true);
  }, [callback, enabled]);
}
