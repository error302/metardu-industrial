/**
 * Keyboard Shortcuts Help — Sprint 12 UI improvement.
 *
 * Press `?` to show all keyboard shortcuts. This is the standard
 * pattern in professional apps (Gmail, GitHub, VS Code, Figma) and
 * is especially important for a survey app where the operator may
 * be wearing gloves or working in low light.
 *
 * Shortcuts are grouped by category. Each shortcut shows the key
 * combination and a description.
 */

import { useState, useEffect } from "react";
import { X, Keyboard } from "lucide-react";
import { colors } from "@/lib/tokens";
import { useEscapeKey } from "@/lib/use-escape-key";

interface ShortcutGroup {
  category: string;
  shortcuts: { keys: string[]; description: string }[];
}

const SHORTCUT_GROUPS: ShortcutGroup[] = [
  {
    category: "Global",
    shortcuts: [
      { keys: ["Ctrl", "K"], description: "Open command palette (fuzzy search all actions)" },
      { keys: ["Ctrl", "Z"], description: "Undo last destructive action" },
      { keys: ["Ctrl", "Y"], description: "Redo (also Ctrl+Shift+Z)" },
      { keys: ["?"], description: "Show this shortcuts help" },
      { keys: ["Esc"], description: "Close dialog / cancel action" },
    ],
  },
  {
    category: "Map",
    shortcuts: [
      { keys: ["+"], description: "Zoom in" },
      { keys: ["-"], description: "Zoom out" },
      { keys: ["F"], description: "Toggle fullscreen" },
      { keys: ["P"], description: "Toggle profile tool (cross-section line)" },
      { keys: ["S"], description: "Toggle live stream (UDP)" },
    ],
  },
  {
    category: "Panels",
    shortcuts: [
      { keys: ["["], description: "Toggle left sidebar" },
      { keys: ["]"], description: "Toggle right panel" },
      { keys: ["1"], description: "Layout: Default" },
      { keys: ["2"], description: "Layout: Data Ingest" },
      { keys: ["3"], description: "Layout: Bathymetry Clean" },
      { keys: ["4"], description: "Layout: Volume Reporting" },
    ],
  },
  {
    category: "File",
    shortcuts: [
      { keys: ["Ctrl", "S"], description: "Save project" },
      { keys: ["Ctrl", "O"], description: "Open project manager" },
      { keys: ["Ctrl", "N"], description: "New project from template" },
    ],
  },
];

interface Props {
  open: boolean;
  onClose: () => void;
}

export function KeyboardShortcutsHelp({ open, onClose }: Props) {
  useEscapeKey(onClose, open);
  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-[60] flex items-center justify-center bg-black/60 backdrop-blur-sm"
      onClick={onClose}
    >
      <div
        onClick={(e) => e.stopPropagation()}
        className="flex max-h-[88vh] w-full max-w-2xl flex-col rounded-lg border border-navy-border bg-navy-panel shadow-2xl"
      >
        <div className="flex items-center justify-between border-b border-navy-border px-5 py-3">
          <h2 className="flex items-center gap-2 text-sm font-semibold text-white">
            <Keyboard className="h-4 w-4" style={{ color: colors.accent }} />
            Keyboard Shortcuts
          </h2>
          <button
            onClick={onClose}
            aria-label="Close"
            title="Close (Esc)"
            className="rounded p-1 text-steel-gray hover:bg-navy-elevated hover:text-white"
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        <div className="flex-1 overflow-y-auto p-5 grid grid-cols-2 gap-6">
          {SHORTCUT_GROUPS.map((group) => (
            <div key={group.category}>
              <h3 className="mb-2 text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
                {group.category}
              </h3>
              <div className="space-y-1.5">
                {group.shortcuts.map((s, i) => (
                  <div key={i} className="flex items-center justify-between gap-2">
                    <span className="text-[11px] text-steel-light">{s.description}</span>
                    <div className="flex gap-1 flex-shrink-0">
                      {s.keys.map((k, j) => (
                        <kbd
                          key={j}
                          className="rounded border px-1.5 py-0.5 font-mono text-[9px] text-white"
                          style={{
                            borderColor: colors.border,
                            background: colors.base,
                            minWidth: k.length > 1 ? "28px" : "20px",
                            textAlign: "center",
                          }}
                        >
                          {k}
                        </kbd>
                      ))}
                    </div>
                  </div>
                ))}
              </div>
            </div>
          ))}
        </div>

        <div className="border-t border-navy-border px-5 py-3 text-center text-[10px] text-steel-gray">
          Shortcuts are skipped when typing in input fields · Press <kbd className="rounded border px-1 py-0.5 font-mono text-[9px]" style={{ borderColor: colors.border }}>?</kbd> anytime to see this help
        </div>
      </div>
    </div>
  );
}

/**
 * Hook that registers the `?` key listener to toggle the shortcuts overlay.
 * Returns `open` + `setOpen` so the caller can also open it from a button.
 */
export function useKeyboardShortcutsHelp() {
  const [open, setOpen] = useState(false);

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement | null;
      const tag = target?.tagName?.toLowerCase();
      const isEditable = tag === "input" || tag === "textarea" || tag === "select" || target?.isContentEditable;
      if (isEditable) return;
      if (e.shiftKey && e.key === "?") {
        e.preventDefault();
        setOpen((v) => !v);
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  return { open, setOpen };
}
