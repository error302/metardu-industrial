/**
 * Customizable Toolbar — Sprint 13.
 *
 * Lets the user pin their most-used actions to a top toolbar for
 * one-click access. Stored in localStorage.
 *
 * Default pinned actions (for a new user):
 *   - Volume Calculator
 *   - Classify Ground (CSF)
 *   - EOM Auditor
 *   - S-44 Compliance
 *   - Settings
 *
 * The user can add/remove actions via the toolbar's "customize" button.
 */

import { create } from "zustand";

export interface ToolbarAction {
  id: string;
  label: string;
  icon: string; // lucide icon name — resolved by the toolbar component
  /** Dialog key to open (matches DialogKey from project-templates). */
  dialogKey: string;
}

const STORAGE_KEY = "metardu-toolbar-actions";

/** All actions that can be pinned to the toolbar. */
export const AVAILABLE_ACTIONS: ToolbarAction[] = [
  // Mining
  { id: "volume_calc", label: "Volume", icon: "Calculator", dialogKey: "volumeCalc" },
  { id: "csf", label: "Classify Ground", icon: "Layers3", dialogKey: "csf" },
  { id: "eom_auditor", label: "EOM Auditor", icon: "ShieldCheck", dialogKey: "eomAuditor" },
  { id: "stockpile_audit", label: "Stockpile Audit", icon: "Boxes", dialogKey: "stockpileAudit" },
  { id: "stockpile_change", label: "Change Detection", icon: "History", dialogKey: "stockpileChange" },
  { id: "blast_report", label: "Blast Report", icon: "Bomb", dialogKey: "blastReport" },
  { id: "highwall", label: "Highwall Monitor", icon: "ShieldAlert", dialogKey: "highwall" },
  { id: "monitoring", label: "4D Monitoring", icon: "Activity", dialogKey: "monitoring" },
  { id: "machine_control", label: "Machine Control", icon: "Cpu", dialogKey: "machineControl" },
  { id: "setout", label: "Setting Out", icon: "Crosshair", dialogKey: "setout" },
  { id: "mine_grid", label: "Mine Grid", icon: "Grid3x3", dialogKey: "mineGrid" },
  { id: "tunnel_profile", label: "Tunnel Profile", icon: "SquareDashed", dialogKey: "tunnelProfile" },
  // Marine
  { id: "cube", label: "CUBE Surface", icon: "Waves", dialogKey: "cube" },
  { id: "s44", label: "S-44 Check", icon: "Shield", dialogKey: "s44" },
  { id: "dredge_audit", label: "Dredge Audit", icon: "Anchor", dialogKey: "dredgeAudit" },
  { id: "cross_section", label: "Cross-Section", icon: "Ruler", dialogKey: "crossSection" },
  { id: "mbes_survey", label: "MBES Reader", icon: "FileSearch", dialogKey: "mbesSurvey" },
  { id: "qc_dashboard", label: "QC Dashboard", icon: "Activity", dialogKey: "qcDashboard" },
  { id: "backscatter", label: "Backscatter", icon: "Grid3x3", dialogKey: "backscatter" },
  { id: "svp", label: "SVP Editor", icon: "Waves", dialogKey: "svp" },
  { id: "tide_gauge", label: "Tide Gauge", icon: "Waves", dialogKey: "tideGauge" },
  // Cross-cutting
  { id: "project", label: "Project Manager", icon: "FolderOpen", dialogKey: "project" },
  { id: "pipeline", label: "Pipeline Editor", icon: "GitBranch", dialogKey: "pipeline" },
  { id: "benchmark", label: "Benchmark", icon: "Gauge", dialogKey: "benchmark" },
  { id: "settings", label: "Settings", icon: "Settings", dialogKey: "settings" },
];

const DEFAULT_PINNED = ["volume_calc", "csf", "eom_auditor", "s44", "settings"];

function loadPinned(): string[] {
  if (typeof window === "undefined") return DEFAULT_PINNED;
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    return raw ? JSON.parse(raw) : DEFAULT_PINNED;
  } catch {
    return DEFAULT_PINNED;
  }
}

function savePinned(ids: string[]) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(ids));
  } catch {
    // Silently fail
  }
}

interface ToolbarState {
  /** IDs of pinned actions, in toolbar order. */
  pinned: string[];
  /** Pin an action. No-op if already pinned. */
  pin: (actionId: string) => void;
  /** Unpin an action. */
  unpin: (actionId: string) => void;
  /** Reorder pinned actions. */
  reorder: (from: number, to: number) => void;
  /** Check if an action is pinned. */
  isPinned: (actionId: string) => boolean;
  /** Reset to defaults. */
  reset: () => void;
}

export const useToolbarStore = create<ToolbarState>((set, get) => ({
  pinned: loadPinned(),

  pin: (actionId) => {
    set((state) => {
      if (state.pinned.includes(actionId)) return state;
      const pinned = [...state.pinned, actionId];
      savePinned(pinned);
      return { pinned };
    });
  },

  unpin: (actionId) => {
    set((state) => {
      const pinned = state.pinned.filter((id) => id !== actionId);
      savePinned(pinned);
      return { pinned };
    });
  },

  reorder: (from, to) => {
    set((state) => {
      const pinned = [...state.pinned];
      const [moved] = pinned.splice(from, 1);
      pinned.splice(to, 0, moved);
      savePinned(pinned);
      return { pinned };
    });
  },

  isPinned: (actionId) => get().pinned.includes(actionId),

  reset: () => {
    savePinned(DEFAULT_PINNED);
    set({ pinned: DEFAULT_PINNED });
  },
}));
