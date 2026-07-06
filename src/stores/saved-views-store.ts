/**
 * Saved View States — Sprint 13.
 *
 * Lets the surveyor save the current map view (extent + zoom + rotation
 * + layer visibility + active domain) under a name, and restore it later.
 *
 * Use cases:
 *   - "Stockpile Pad A" — quick zoom to the stockpile yard
 *   - "Pit Bench 1050" — the current active mining bench
 *   - "Channel Centerline" — the dredge survey alignment
 *   - "Pre-survey Overview" — the full survey area before starting
 *
 * Stored in localStorage. Up to 20 saved views. Each view stores:
 *   - name, timestamp
 *   - map center [lon, lat], zoom, rotation
 *   - layer visibility map
 *   - active domain (mining/marine/both)
 */

import { create } from "zustand";

export interface SavedView {
  id: string;
  name: string;
  timestamp: number;
  /** Map center [x, y] in the active CRS. */
  center: [number, number];
  /** Map zoom level. */
  zoom: number;
  /** Map rotation in radians. */
  rotation: number;
  /** Layer visibility state at save time. */
  layers: Record<string, boolean>;
  /** Active domain. */
  domain: "mining" | "marine" | "both";
  /** Active EPSG. */
  epsg: string;
}

const STORAGE_KEY = "metardu-saved-views";
const MAX_VIEWS = 20;

function loadViews(): SavedView[] {
  if (typeof window === "undefined") return [];
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    return raw ? JSON.parse(raw) : [];
  } catch {
    return [];
  }
}

function saveViews(views: SavedView[]) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(views));
  } catch {
    // localStorage full or unavailable — silently fail
  }
}

interface SavedViewState {
  views: SavedView[];
  /** Save the current view state. Returns the saved view. */
  save: (view: Omit<SavedView, "id" | "timestamp">) => SavedView;
  /** Delete a saved view by ID. */
  remove: (id: string) => void;
  /** Rename a saved view. */
  rename: (id: string, name: string) => void;
  /** Get a saved view by ID. */
  get: (id: string) => SavedView | undefined;
  /** Clear all saved views. */
  clear: () => void;
}

export const useSavedViewsStore = create<SavedViewState>((set, get) => ({
  views: loadViews(),

  save: (view) => {
    const saved: SavedView = {
      ...view,
      id: `view-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
      timestamp: Date.now(),
    };
    set((state) => {
      const views = [saved, ...state.views].slice(0, MAX_VIEWS);
      saveViews(views);
      return { views };
    });
    return saved;
  },

  remove: (id) => {
    set((state) => {
      const views = state.views.filter((v) => v.id !== id);
      saveViews(views);
      return { views };
    });
  },

  rename: (id, name) => {
    set((state) => {
      const views = state.views.map((v) => (v.id === id ? { ...v, name } : v));
      saveViews(views);
      return { views };
    });
  },

  get: (id) => get().views.find((v) => v.id === id),

  clear: () => {
    saveViews([]);
    set({ views: [] });
  },
}));
