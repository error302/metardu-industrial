/**
 * Undo/Redo Store — Sprint 11 Quality of Life #2.
 *
 * Global undo stack that wraps every destructive operation in the app.
 * Each entry is `{ description, undo, redo }`. Ctrl+Z pops the undo
 * stack and runs the entry's `undo` callback; Ctrl+Y re-applies via
 * the redo stack. The stack is capped at 100 entries to bound memory.
 *
 * Usage from any component:
 *
 * ```ts
 * const push = useUndoStore(s => s.push);
 * push({
 *   description: "Remove file stockpile.las",
 *   undo: () => reAddFile("stockpile.las"),
 *   redo: () => removeFile("stockpile.las"),
 * });
 * ```
 *
 * The workspace shell registers global Ctrl+Z / Ctrl+Y listeners.
 * The status bar shows the stack depth and the next operation's
 * description as a hover tooltip.
 */

import { create } from "zustand";

export interface UndoAction {
  /** Human-readable description shown in tooltips and history. */
  description: string;
  /** Revert the action. */
  undo: () => void | Promise<void>;
  /** Re-apply the action (after undo). */
  redo: () => void | Promise<void>;
  /** Optional category for grouping/filtering in the history view. */
  category?: string;
}

interface UndoState {
  /** Undo stack — newest at the end. */
  undoStack: UndoAction[];
  /** Redo stack — newest at the end. Cleared on every new push. */
  redoStack: UndoAction[];
  /** Whether an undo/redo operation is currently executing (prevents re-entry). */
  isExecuting: boolean;
  /** Last action description for the status bar tooltip. */
  lastDescription: string | null;
  /** Max stack size (default 100). Older entries are dropped. */
  maxStack: number;

  /** Push a new undoable action. Clears the redo stack. */
  push: (action: UndoAction) => void;
  /** Undo the most recent action. Returns true if an action was undone. */
  undo: () => Promise<boolean>;
  /** Redo the most recently undone action. Returns true if an action was redone. */
  redo: () => Promise<boolean>;
  /** Clear both stacks (e.g., on project change). */
  clear: () => void;
  /** Convenience getters. */
  canUndo: () => boolean;
  canRedo: () => boolean;
  /** Peek at the next action to be undone (for tooltips). */
  peekUndo: () => UndoAction | null;
  /** Peek at the next action to be redone. */
  peekRedo: () => UndoAction | null;
}

export const useUndoStore = create<UndoState>((set, get) => ({
  undoStack: [],
  redoStack: [],
  isExecuting: false,
  lastDescription: null,
  maxStack: 100,

  push: (action) => {
    if (get().isExecuting) return; // Don't push while undoing/redoing
    set((state) => {
      const newUndo = [...state.undoStack, action];
      // Cap the stack — drop oldest entries
      if (newUndo.length > state.maxStack) {
        newUndo.splice(0, newUndo.length - state.maxStack);
      }
      return {
        undoStack: newUndo,
        redoStack: [], // Clear redo on new action
        lastDescription: action.description,
      };
    });
  },

  undo: async () => {
    const state = get();
    if (state.isExecuting || state.undoStack.length === 0) return false;
    set({ isExecuting: true });
    try {
      const action = state.undoStack[state.undoStack.length - 1];
      await action.undo();
      set((s) => ({
        undoStack: s.undoStack.slice(0, -1),
        redoStack: [...s.redoStack, action],
        lastDescription: action.description,
      }));
      return true;
    } finally {
      set({ isExecuting: false });
    }
  },

  redo: async () => {
    const state = get();
    if (state.isExecuting || state.redoStack.length === 0) return false;
    set({ isExecuting: true });
    try {
      const action = state.redoStack[state.redoStack.length - 1];
      await action.redo();
      set((s) => ({
        undoStack: [...s.undoStack, action],
        redoStack: s.redoStack.slice(0, -1),
        lastDescription: action.description,
      }));
      return true;
    } finally {
      set({ isExecuting: false });
    }
  },

  clear: () => {
    set({ undoStack: [], redoStack: [], lastDescription: null });
  },

  canUndo: () => get().undoStack.length > 0,
  canRedo: () => get().redoStack.length > 0,

  peekUndo: () => {
    const stack = get().undoStack;
    return stack.length > 0 ? stack[stack.length - 1] : null;
  },
  peekRedo: () => {
    const stack = get().redoStack;
    return stack.length > 0 ? stack[stack.length - 1] : null;
  },
}));

/**
 * Convenience hook for components that just want to push an undoable
 * action without subscribing to the full state.
 *
 * ```ts
 * const pushUndo = usePushUndo();
 * pushUndo({ description: "...", undo: ..., redo: ... });
 * ```
 */
export function usePushUndo() {
  return useUndoStore((s) => s.push);
}
