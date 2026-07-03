/**
 * File Picker — native OS file browser using Tauri's dialog plugin.
 *
 * Surveyors need to click "Browse..." not type file paths.
 * This utility provides a simple async function that opens the OS
 * file picker and returns the selected path (or null if cancelled).
 *
 * Usage:
 *   import { pickFile, pickFolder } from "@/lib/file-picker";
 *
 *   const path = await pickFile({ extensions: ["tif", "tiff"] });
 *   if (path) { /* use the path *\/ }
 */

import { open, save } from "@tauri-apps/plugin-dialog";
import { isTauri } from "@tauri-apps/api/core";

export interface FilePickerOptions {
  /** File extensions to filter (without dot, e.g., ["tif", "tiff"]) */
  extensions?: string[];
  /** Display name for the filter (e.g., "GeoTIFF DEM") */
  filterName?: string;
  /** Dialog title */
  title?: string;
}

/**
 * Open the OS file picker and return the selected file path.
 * Returns null if the user cancels.
 */
export async function pickFile(options: FilePickerOptions = {}): Promise<string | null> {
  if (!isTauri()) return null;

  const filters = options.extensions
    ? [{
        name: options.filterName ?? `${options.extensions.join(", ").toUpperCase()} files`,
        extensions: options.extensions,
      }]
    : undefined;

  const result = await open({
    title: options.title ?? "Select file",
    filters,
    multiple: false,
    directory: false,
  });

  // open() returns string | string[] | null when multiple=false it returns string | null
  return typeof result === "string" ? result : null;
}

/**
 * Open the OS folder picker and return the selected folder path.
 * Returns null if the user cancels.
 */
export async function pickFolder(title?: string): Promise<string | null> {
  if (!isTauri()) return null;

  const result = await open({
    title: title ?? "Select folder",
    directory: true,
    multiple: false,
  });

  return typeof result === "string" ? result : null;
}

/**
 * Open the OS save-file picker and return the chosen path.
 * Returns null if the user cancels.
 */
export async function pickSaveFile(options: FilePickerOptions = {}): Promise<string | null> {
  if (!isTauri()) return null;

  const filters = options.extensions
    ? [{
        name: options.filterName ?? `${options.extensions.join(", ").toUpperCase()} files`,
        extensions: options.extensions,
      }]
    : undefined;

  const result = await save({
    title: options.title ?? "Save file",
    filters,
  });

  return result ?? null;
}
