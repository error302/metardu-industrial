/**
 * CRS registry — wraps proj4js to register coordinate systems on demand.
 *
 * Per ARCHITECTURE.md §8.3 — proj4js integration is what makes mine grids
 * and marine datums first-class. EPSG:4326 and EPSG:3857 are built into
 * OpenLayers; everything else must be registered before use.
 *
 * We fetch proj4 definitions from epsg.io on first use and cache them
 * in localStorage so subsequent loads are offline-capable.
 */

import proj4 from "proj4";
import { get as olGetProjection } from "ol/proj";
import { register } from "ol/proj/proj4";

let registered = new Set<string>(["EPSG:4326", "EPSG:3857"]);
let initPromise: Promise<void> | null = null;

/**
 * One-time setup: register proj4 with OpenLayers. Must be called before
 * any map view uses a non-built-in projection.
 */
export function initProj4(): void {
  if (initPromise) return;
  register(proj4);
  initPromise = Promise.resolve();
}

/**
 * Register a CRS by EPSG code. Fetches the proj4 definition from epsg.io
 * if not already cached. After registration, the projection is usable
 * in OpenLayers views, MousePosition controls, etc.
 *
 * @returns The registered proj4js definition string
 */
export async function registerEpsg(epsg: string): Promise<string> {
  initProj4();

  if (registered.has(epsg)) {
    return localStorage.getItem(`proj4.${epsg}`) ?? "";
  }

  // Check cache first
  const cached = localStorage.getItem(`proj4.${epsg}`);
  if (cached) {
    proj4.defs(epsg, cached);
    registered.add(epsg);
    syncToOpenLayers(epsg);
    return cached;
  }

  // Fetch from epsg.io — the canonical source for proj4 string definitions
  const url = `https://epsg.io/${epsg.replace("EPSG:", "")}.proj4`;
  const res = await fetch(url);
  if (!res.ok) {
    throw new Error(`Failed to fetch proj4 definition for ${epsg}: HTTP ${res.status}`);
  }
  const def = (await res.text()).trim();

  if (!def || def.startsWith("<") || def.length < 10) {
    throw new Error(`Invalid proj4 definition for ${epsg}: ${def.slice(0, 100)}`);
  }

  proj4.defs(epsg, def);
  localStorage.setItem(`proj4.${epsg}`, def);
  registered.add(epsg);
  syncToOpenLayers(epsg);
  return def;
}

/**
 * Register a custom CRS via a proj4 definition string (e.g., a mine grid).
 * Used by Settings → Coordinate Systems to add local engineering CRSs.
 */
export function registerCustomProj4(
  code: string,
  def: string,
): void {
  initProj4();
  proj4.defs(code, def);
  localStorage.setItem(`proj4.${code}`, def);
  registered.add(code);
  syncToOpenLayers(code);
}

/**
 * Get the OpenLayers projection object for a registered CRS.
 * Returns undefined if not yet registered.
 */
export function getOlProjection(epsg: string) {
  if (!registered.has(epsg)) return undefined;
  return olGetProjection(epsg);
}

/**
 * Check if a CRS is registered.
 */
export function isRegistered(epsg: string): boolean {
  return registered.has(epsg);
}

/**
 * Transform a coordinate from one CRS to another.
 * Both must be registered first.
 */
export function transform(
  coord: [number, number],
  from: string,
  to: string,
): [number, number] {
  // proj4 accepts code strings directly when defs() has been registered
  return proj4(from, to, coord) as unknown as [number, number];
}

// ──────────────────────────────────────────────────────────────────
// Internal — sync proj4 definition to OpenLayers

function syncToOpenLayers(code: string): void {
  // OL's register(proj4) hooks up the proj4 instance, but OL needs to
  // build its own Projection object from the proj4.defs entry. Calling
  // olGetProjection(code) after registering forces this lazy build.
  // If the projection isn't found, OL builds it lazily on first use.
  void olGetProjection(code);
}
