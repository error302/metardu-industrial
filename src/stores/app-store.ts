/**
 * MetaRDU Industrial — App Store
 * Holds app-level state: boot phase, active workspace, settings.
 */

import { create } from "zustand";
import type { DomainMode } from "@/lib/tokens";

export type BootPhase =
  | "splash"
  | "modules"
  | "onboarding"
  | "workspace"
  | "project-loading";

export interface AppSettings {
  defaultDomain: DomainMode;
  defaultEpsg: string;
  density: "compact" | "comfortable";
  reducedMotion: boolean;
  theme: "dark" | "light";
}

interface AppState {
  phase: BootPhase;
  activeDomain: DomainMode;
  settings: AppSettings;
  hasCompletedOnboarding: boolean;

  setPhase: (phase: BootPhase) => void;
  setActiveDomain: (domain: DomainMode) => void;
  completeOnboarding: (settings: Partial<AppSettings>) => void;
  updateSettings: (patch: Partial<AppSettings>) => void;
}

const DEFAULT_SETTINGS: AppSettings = {
  defaultDomain: "both",
  defaultEpsg: "EPSG:4326",
  density: "comfortable",
  reducedMotion: false,
  theme: "dark",
};

export const useAppStore = create<AppState>((set) => ({
  phase: "splash",
  activeDomain: "both",
  settings: DEFAULT_SETTINGS,
  hasCompletedOnboarding: false,

  setPhase: (phase) => set({ phase }),
  setActiveDomain: (domain) => set({ activeDomain: domain }),

  completeOnboarding: (settings) =>
    set((state) => ({
      hasCompletedOnboarding: true,
      settings: { ...state.settings, ...settings },
      activeDomain: settings.defaultDomain ?? state.settings.defaultDomain,
      phase: "workspace",
    })),

  updateSettings: (patch) =>
    set((state) => ({ settings: { ...state.settings, ...patch } })),
}));
