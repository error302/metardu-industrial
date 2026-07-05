/**
 * Shared CRS quickpicks — single source of truth for the onboarding screen
 * and the Settings dialog.
 *
 * IMPORTANT — datum compliance notes:
 *
 *   GDA94 vs GDA2020 (Australia):
 *     EPSG:28354/55/56 are GDA94 / MGA zones. Mining survey plans in
 *     Australia dated AFTER September 2022 are LEGALLY REQUIRED to
 *     reference GDA2020 (EPSG:7854/7855/7856). Plans from the transition
 *     window had to note either. Silently defaulting to GDA94 today is
 *     a compliance failure for the customer, not a UI preference.
 *
 *     We surface BOTH but label them clearly so the surveyor cannot
 *     accidentally pick the obsolete one. The label always carries the
 *     datum name + the legally-correct status.
 *
 *   NAD83 vs NAD83(2011):
 *     EPSG:4269 is NAD83 (generic). For US mining work post-2011 the
 *     legally-current realization is NAD83(2011) — EPSG:6318. We surface
 *     both.
 *
 *   ETRS89 vs national realizations:
 *     Europe: ETRS89 (EPSG:4258) is the continental standard. National
 *     realizations (e.g. RGF93 in France, ETRS89 in Spain) are kept
 *     because local regulators often still reference them.
 *
 * If you add a country-specific entry here, also add it to the Rust-side
 * CRS lookup if reprojection is needed (src-tauri/src/geodesy/mod.rs).
 */

export interface CrsEntry {
  /** EPSG code with the "EPSG:" prefix, e.g. "EPSG:7854". */
  code: string;
  /** Human-readable label shown in the dropdown. */
  label: string;
  /** Datum / realization name — shown as a sub-label for disambiguation. */
  datum: string;
  /** Region tag for filtering. */
  region: "Global" | "Australia" | "North America" | "Europe" | "Africa" | "South America" | "Asia";
  /** True if this is the current legally-mandated datum for new surveys
   *  in this region. Drives the "CURRENT" badge in the UI. */
  current?: boolean;
  /** True if this is a superseded datum that's still selectable for
   *  historical data. Drives the "LEGACY" badge + warning. */
  legacy?: boolean;
  /** Default coordinate epoch (decimal year) for dynamic CRSs.
   *  Undefined for static CRSs. */
  epoch?: number;
}

export const CRS_QUICKPICKS: CrsEntry[] = [
  // ── Global ───────────────────────────────────────────────────────
  {
    code: "EPSG:4326",
    label: "WGS 84 (geographic)",
    datum: "WGS 84",
    region: "Global",
    current: true,
  },
  {
    code: "EPSG:3857",
    label: "Web Mercator (display only — not for survey plans)",
    datum: "WGS 84 / Pseudo-Mercator",
    region: "Global",
  },

  // ── Australia — GDA2020 is the legally-current datum ──────────────
  // Source: ICSM, "GDA2020 became the official national datum on
  // 1 January 2020. From 1 September 2022, all mining survey plans
  // must reference GDA2020."
  {
    code: "EPSG:7844",
    label: "GDA2020 (geographic) — CURRENT",
    datum: "GDA2020",
    region: "Australia",
    current: true,
    epoch: 2020.0,
  },
  {
    code: "EPSG:7854",
    label: "GDA2020 / MGA Zone 54 — CURRENT",
    datum: "GDA2020",
    region: "Australia",
    current: true,
    epoch: 2020.0,
  },
  {
    code: "EPSG:7855",
    label: "GDA2020 / MGA Zone 55 — CURRENT",
    datum: "GDA2020",
    region: "Australia",
    current: true,
    epoch: 2020.0,
  },
  {
    code: "EPSG:7856",
    label: "GDA2020 / MGA Zone 56 — CURRENT",
    datum: "GDA2020",
    region: "Australia",
    current: true,
    epoch: 2020.0,
  },
  // Legacy GDA94 entries — kept for working with historical survey data.
  // The label explicitly flags them as LEGACY so a surveyor can't
  // accidentally pick them for a new plan.
  {
    code: "EPSG:28354",
    label: "GDA94 / MGA Zone 54 — LEGACY (pre-2022)",
    datum: "GDA94",
    region: "Australia",
    legacy: true,
  },
  {
    code: "EPSG:28355",
    label: "GDA94 / MGA Zone 55 — LEGACY (pre-2022)",
    datum: "GDA94",
    region: "Australia",
    legacy: true,
  },
  {
    code: "EPSG:28356",
    label: "GDA94 / MGA Zone 56 — LEGACY (pre-2022)",
    datum: "GDA94",
    region: "Australia",
    legacy: true,
  },

  // ── North America ────────────────────────────────────────────────
  {
    code: "EPSG:4269",
    label: "NAD83 (geographic) — legacy generic",
    datum: "NAD83",
    region: "North America",
    legacy: true,
  },
  {
    code: "EPSG:6318",
    label: "NAD83(2011) (geographic) — CURRENT",
    datum: "NAD83 (2011)",
    region: "North America",
    current: true,
    epoch: 2010.0,
  },
  {
    code: "EPSG:6350",
    label: "NAD83(2011) / UTM Zone 10N — CURRENT (Pacific NW)",
    datum: "NAD83 (2011)",
    region: "North America",
    current: true,
    epoch: 2010.0,
  },

  // ── Europe ───────────────────────────────────────────────────────
  {
    code: "EPSG:4258",
    label: "ETRS89 (geographic) — CURRENT for pan-EU",
    datum: "ETRS89",
    region: "Europe",
    current: true,
    epoch: 1989.0,
  },
  {
    code: "EPSG:2154",
    label: "RGF93 / Lambert-93 (France) — CURRENT",
    datum: "RGF93 / Lambert-93",
    region: "Europe",
    current: true,
    epoch: 2009.0,
  },

  // ── Africa (UTM zones) ───────────────────────────────────────────
  // South Africa uses the Hartebeesthoek94 / Lo. system (EPSG:2046+
  // and 9221+ for the newer version), but most SA mines operate on
  // local mine grids via a custom proj4 string (registered through
  // the Settings → Custom CRS dialog). The UTM zones below cover
  // common African mining jurisdictions.
  {
    code: "EPSG:32733",
    label: "UTM Zone 33S (southern Africa, e.g. Zambia)",
    datum: "WGS 84",
    region: "Africa",
    current: true,
  },
  {
    code: "EPSG:32734",
    label: "UTM Zone 34S (South Africa, Botswana)",
    datum: "WGS 84",
    region: "Africa",
    current: true,
  },
  {
    code: "EPSG:32735",
    label: "UTM Zone 35S (Mozambique, Zimbabwe)",
    datum: "WGS 84",
    region: "Africa",
    current: true,
  },
  {
    code: "EPSG:32736",
    label: "UTM Zone 36S (Tanzania, DRC east)",
    datum: "WGS 84",
    region: "Africa",
    current: true,
  },

  // ── South America ────────────────────────────────────────────────
  {
    code: "EPSG:32719",
    label: "UTM Zone 19S (Chile, Peru, Bolivia)",
    datum: "WGS 84",
    region: "South America",
    current: true,
  },
  {
    code: "EPSG:32718",
    label: "UTM Zone 18S (Colombia, Ecuador, Peru)",
    datum: "WGS 84",
    region: "South America",
    current: true,
  },

  // ── Asia (mining hotspots) ───────────────────────────────────────
  {
    code: "EPSG:32650",
    label: "UTM Zone 50N (Mongolia, Western Australia off-coast)",
    datum: "WGS 84",
    region: "Asia",
    current: true,
  },
  {
    code: "EPSG:32647",
    label: "UTM Zone 47N (Indonesia, Thailand)",
    datum: "WGS 84",
    region: "Asia",
    current: true,
  },
];

/**
 * Filter the quickpicks by free-text query. Matches code OR label OR
 * datum OR region. Case-insensitive.
 */
export function filterCrsQuickpicks(query: string): CrsEntry[] {
  const q = query.trim().toLowerCase();
  if (!q) return CRS_QUICKPICKS;
  return CRS_QUICKPICKS.filter(
    (c) =>
      c.code.toLowerCase().includes(q) ||
      c.label.toLowerCase().includes(q) ||
      c.datum.toLowerCase().includes(q) ||
      c.region.toLowerCase().includes(q),
  );
}

/** Get a CrsEntry by EPSG code. Returns undefined if not in the quickpicks. */
export function getCrsEntry(code: string): CrsEntry | undefined {
  return CRS_QUICKPICKS.find((c) => c.code === code);
}

/** Format the datum + epoch note that should appear on every survey report.
 *  e.g. "Datum: GDA2020 / Epoch 2020.0" or "Datum: WGS 84" (no epoch for static). */
export function formatDatumNote(code: string): string {
  const entry = getCrsEntry(code);
  if (!entry) return code; // unknown — just show the code
  const epochPart = entry.epoch ? ` / Epoch ${entry.epoch.toFixed(1)}` : "";
  return `Datum: ${entry.datum}${epochPart}`;
}
