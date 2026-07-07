/**
 * MetaRDU Industrial — App Store
 * Holds app-level state: boot phase, active workspace, settings.
 *
 * Persistence model:
 *   - The 4 Rust-backed fields (defaultDomain, defaultEpsg, density,
 *     reducedMotion) are written via the `save_settings` Tauri command
 *     to `app_config_dir/settings.json`. In browser mode they fall back
 *     to `localStorage["metardu.settings"]`.
 *   - The frontend-only `theme` field is always persisted to
 *     `localStorage["metardu.theme"]` (works in both browser & Tauri
 *     webview modes).
 *   - The "skip onboarding on subsequent boots" flag is persisted to
 *     `localStorage["metardu.onboarded"]` so the splash → modules →
 *     workspace path is taken after the first run.
 *
 * `hydrate()` is called once from `App.tsx` on mount to load everything
 * back into the store before the boot sequence decides which screen to
 * show. Without this, every cold boot would reset the user's saved
 * preferences — a real bug that was silently losing user settings.
 */

import { create } from "zustand";
import type { DomainMode } from "@/lib/tokens";
import { getSettings, isNative, type AppSettingsRpc } from "@/lib/tauri-ipc";

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
  /** True once `hydrate()` has finished loading persisted state. */
  hydrated: boolean;

  setPhase: (phase: BootPhase) => void;
  setActiveDomain: (domain: DomainMode) => void;
  completeOnboarding: (settings: Partial<AppSettings>) => void;
  updateSettings: (patch: Partial<AppSettings>) => void;
  /** Load persisted settings + onboarding flag into the store. */
  hydrate: () => Promise<void>;
}

const DEFAULT_SETTINGS: AppSettings = {
  defaultDomain: "both",
  defaultEpsg: "EPSG:4326",
  density: "comfortable",
  reducedMotion: false,
  theme: "dark",
};

const THEME_KEY = "metardu.theme";
const ONBOARDED_KEY = "metardu.onboarded";
const BROWSER_SETTINGS_KEY = "metardu.settings";

/** Type guard for the persisted theme string. */
function readPersistedTheme(): "dark" | "light" | null {
  try {
    const raw = localStorage.getItem(THEME_KEY);
    return raw === "dark" || raw === "light" ? raw : null;
  } catch {
    return null;
  }
}

/** Type guard for persisted onboarding flag. */
function readOnboardedFlag(): boolean {
  try {
    return localStorage.getItem(ONBOARDED_KEY) === "1";
  } catch {
    return false;
  }
}

/** Coerce a raw persisted domain string into the typed DomainMode. */
function coerceDomain(value: string | undefined): DomainMode | null {
  if (value === "mining" || value === "marine" || value === "both") return value;
  return null;
}

function coerceDensity(value: string | undefined): "compact" | "comfortable" | null {
  return value === "compact" || value === "comfortable" ? value : null;
}

export const useAppStore = create<AppState>((set, get) => ({
  phase: "splash",
  activeDomain: "both",
  settings: DEFAULT_SETTINGS,
  hasCompletedOnboarding: false,
  hydrated: false,

  setPhase: (phase) => set({ phase }),
  setActiveDomain: (domain) => set({ activeDomain: domain }),

  completeOnboarding: (settings) => {
    set((state) => ({
      hasCompletedOnboarding: true,
      settings: { ...state.settings, ...settings },
      activeDomain: settings.defaultDomain ?? state.settings.defaultDomain,
      phase: "workspace",
    }));
    // Persist the "skip onboarding next time" flag + theme so a cold
    // boot honours the user's previous choice. Theme is written here
    // too because the onboarding screen lets the user pick a domain
    // but not a theme — defaults are used, and we still want the flag.
    try {
      localStorage.setItem(ONBOARDED_KEY, "1");
      const theme = get().settings.theme;
      localStorage.setItem(THEME_KEY, theme);
    } catch {
      // localStorage may be unavailable (private mode, sandbox) — non-fatal
    }
  },

  updateSettings: (patch) => {
    set((state) => ({ settings: { ...state.settings, ...patch } }));
    // Theme is frontend-only — persist it immediately so it survives
    // cold boots even if the Rust save_settings call is never made.
    if (patch.theme === "dark" || patch.theme === "light") {
      try {
        localStorage.setItem(THEME_KEY, patch.theme);
      } catch {
        // non-fatal
      }
    }
  },

  hydrate: async () => {
    // 1. Always load theme from localStorage first — it's frontend-only
    //    and works the same in browser & Tauri webview modes.
    const persistedTheme = readPersistedTheme();
    const onboardedFlag = readOnboardedFlag();

    let rustSettings: AppSettingsRpc | null = null;
    let browserSettings: AppSettingsRpc | null = null;

    // 2. Try the Rust side first (returns defaults if no file exists yet).
    if (isNative()) {
      try {
        rustSettings = await getSettings();
      } catch {
        // Rust command failed (corrupted file, permission error, etc.)
        // — fall through to browser fallback below.
        rustSettings = null;
      }
    }

    // 3. Browser fallback: read the localStorage blob that
    //    `saveSettings()` writes when running outside Tauri.
    if (!rustSettings) {
      try {
        const raw = localStorage.getItem(BROWSER_SETTINGS_KEY);
        if (raw) {
          const parsed = JSON.parse(raw) as Partial<AppSettingsRpc>;
          browserSettings = {
            defaultDomain: parsed.defaultDomain ?? DEFAULT_SETTINGS.defaultDomain,
            defaultEpsg: parsed.defaultEpsg ?? DEFAULT_SETTINGS.defaultEpsg,
            density: parsed.density ?? DEFAULT_SETTINGS.density,
            reducedMotion: parsed.reducedMotion ?? DEFAULT_SETTINGS.reducedMotion,
          };
        }
      } catch {
        // Corrupted localStorage blob — ignore, use defaults.
      }
    }

    const source = rustSettings ?? browserSettings;
    const nextSettings: AppSettings = { ...DEFAULT_SETTINGS };
    if (source) {
      const domain = coerceDomain(source.defaultDomain);
      if (domain) nextSettings.defaultDomain = domain;
      if (typeof source.defaultEpsg === "string" && source.defaultEpsg) {
        nextSettings.defaultEpsg = source.defaultEpsg;
      }
      const density = coerceDensity(source.density);
      if (density) nextSettings.density = density;
      if (typeof source.reducedMotion === "boolean") {
        nextSettings.reducedMotion = source.reducedMotion;
      }
    }
    if (persistedTheme) nextSettings.theme = persistedTheme;

    // Sprint 20 — check if the user has created an account (is_onboarded_cmd).
    // If they haven't, force the onboarding phase so the AccountDialog shows.
    let accountOnboarded = true; // default: assume onboarded (browser mode)
    if (isNative()) {
      try {
        const { invoke } = await import("@tauri-apps/api/core");
        accountOnboarded = await invoke<boolean>("is_onboarded_cmd");
      } catch {
        // If the command fails, fall through to the old behavior
        accountOnboarded = true;
      }
    }

    const hasCompletedOldOnboarding =
      Boolean(rustSettings) || Boolean(browserSettings) || onboardedFlag;
    const fullyOnboarded = hasCompletedOldOnboarding && accountOnboarded;

    set({
      settings: nextSettings,
      activeDomain: nextSettings.defaultDomain,
      // Skip onboarding only if BOTH the old onboarding (settings) AND
      // the new account onboarding (profile.json) are complete.
      hasCompletedOnboarding: fullyOnboarded,
      hydrated: true,
    });
  },
}));
