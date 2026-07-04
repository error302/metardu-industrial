/**
 * File Picker — native OS file browser using Tauri's dialog plugin.
 *
 * Surveyors need to click "Browse..." not type file paths.
 * This utility provides a simple async function that opens the OS
 * file picker and returns the selected path (or null if cancelled).
 *
 * Gracefully degrades: if the dialog plugin isn't available (e.g., in
 * browser dev mode), returns null instead of crashing the app.
 */

import { isTauri } from "@tauri-apps/api/core";

export interface FilePickerOptions {
  /** File extensions to filter (without dot, e.g., ["tif", "tiff"]) */
  extensions?: string[];
  /** Display name for the filter (e.g., "GeoTIFF DEM") */
  filterName?: string;
  /** Dialog title */
  title?: string;
}

// Lazy-load the dialog plugin — avoids crashing if not installed
let dialogOpen: ((opts: unknown) => Promise<string | string[] | null>) | null = null;
let dialogSave: ((opts: unknown) => Promise<string | null>) | null = null;

async function ensureDialogLoaded(): Promise<boolean> {
  if (dialogOpen && dialogSave) return true;
  try {
    const mod = await import("@tauri-apps/plugin-dialog");
    dialogOpen = mod.open as typeof dialogOpen;
    dialogSave = mod.save as typeof dialogSave;
    return true;
  } catch {
    console.warn("@tauri-apps/plugin-dialog not available — file picker disabled");
    return false;
  }
}

/**
 * Open the OS file picker and return the selected file path.
 * Returns null if the user cancels or the plugin is unavailable.
 */
export async function pickFile(options: FilePickerOptions = {}): Promise<string | null> {
  if (!isTauri()) return null;
  if (!(await ensureDialogLoaded())) return null;

  const filters = options.extensions
    ? [{
        name: options.filterName ?? `${options.extensions.join(", ").toUpperCase()} files`,
        extensions: options.extensions,
      }]
    : undefined;

  try {
    const result = await dialogOpen!({
      title: options.title ?? "Select file",
      filters,
      multiple: false,
      directory: false,
    });
    return typeof result === "string" ? result : null;
  } catch {
    return null;
  }
}

/**
 * Open the OS folder picker and return the selected folder path.
 * Returns null if the user cancels or the plugin is unavailable.
 */
export async function pickFolder(title?: string): Promise<string | null> {
  if (!isTauri()) return null;
  if (!(await ensureDialogLoaded())) return null;

  try {
    const result = await dialogOpen!({
      title: title ?? "Select folder",
      directory: true,
      multiple: false,
    });
    return typeof result === "string" ? result : null;
  } catch {
    return null;
  }
}

/**
 * Open the OS save-file picker and return the chosen path.
 * Returns null if the user cancels or the plugin is unavailable.
 */
export async function pickSaveFile(options: FilePickerOptions = {}): Promise<string | null> {
  if (!isTauri()) return null;
  if (!(await ensureDialogLoaded())) return null;

  const filters = options.extensions
    ? [{
        name: options.filterName ?? `${options.extensions.join(", ").toUpperCase()} files`,
        extensions: options.extensions,
      }]
    : undefined;

  try {
    const result = await dialogSave!({
      title: options.title ?? "Save file",
      filters,
    });
    return result ?? null;
  } catch {
    return null;
  }
}
