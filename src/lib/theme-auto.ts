/**
 * Theme Auto-Switch — Sprint 13.
 *
 * Automatically switches between dark (cabin) and light (daylight) themes
 * based on the user's local sunrise/sunset time. The surveyor working
 * outdoors at noon gets the high-contrast light theme; the surveyor in
 * a dim survey cabin at night gets the dark theme.
 *
 * The user can override the auto-switch in Settings (Manual / Auto / Auto+Offset).
 * The override is persisted in localStorage.
 *
 * Sunrise/sunset calculation uses a simplified NOAA model — good enough
 * for theme switching (not for navigation). No external API call.
 */

import { useEffect, useState, useCallback } from "react";

export type ThemeMode = "dark" | "light" | "auto";
export type ResolvedTheme = "dark" | "light";

const STORAGE_KEY = "metardu-theme-mode";
const LAT_KEY = "metardu-theme-lat";
const LON_KEY = "metardu-theme-lon";

/** Get the user's saved theme mode (default: "auto"). */
export function getThemeMode(): ThemeMode {
  if (typeof window === "undefined") return "auto";
  return (localStorage.getItem(STORAGE_KEY) as ThemeMode) || "auto";
}

/** Save the user's theme mode preference. */
export function setThemeMode(mode: ThemeMode) {
  localStorage.setItem(STORAGE_KEY, mode);
  window.dispatchEvent(new CustomEvent("metardu-theme-change", { detail: mode }));
}

/** Get the user's saved location for sunrise/sunset calculation. */
export function getSavedLocation(): { lat: number; lon: number } | null {
  const lat = localStorage.getItem(LAT_KEY);
  const lon = localStorage.getItem(LON_KEY);
  if (lat && lon) {
    return { lat: parseFloat(lat), lon: parseFloat(lon) };
  }
  return null;
}

/** Save the user's location for sunrise/sunset calculation. */
export function setSavedLocation(lat: number, lon: number) {
  localStorage.setItem(LAT_KEY, String(lat));
  localStorage.setItem(LON_KEY, String(lon));
}

/**
 * Calculate sunrise/sunset times for a given date + location.
 *
 * Uses the NOAA solar calculation algorithm (simplified).
 * Returns times in minutes from midnight (local solar time).
 * Returns null if the sun doesn't rise/set (polar regions in summer/winter).
 *
 * Reference: https://gml.noaa.gov/grad/solcalc/calcdetails.html
 */
export function calculateSunriseSunset(
  date: Date,
  lat: number,
  lon: number,
): { sunrise: number | null; sunset: number | null } {
  // Day of year
  const start = new Date(date.getFullYear(), 0, 0);
  const diff = date.getTime() - start.getTime();
  const dayOfYear = Math.floor(diff / (1000 * 60 * 60 * 24));

  // Solar declination (degrees)
  const decl = 23.45 * Math.sin((2 * Math.PI * (284 + dayOfYear)) / 365);

  // Hour angle at sunrise/sunset (degrees)
  const latRad = (lat * Math.PI) / 180;
  const declRad = (decl * Math.PI) / 180;
  const cosHourAngle =
    (Math.cos((90.833 * Math.PI) / 180) - Math.sin(latRad) * Math.sin(declRad)) /
    (Math.cos(latRad) * Math.cos(declRad));

  if (cosHourAngle > 1) {
    // Sun never rises (polar winter)
    return { sunrise: null, sunset: null };
  }
  if (cosHourAngle < -1) {
    // Sun never sets (polar summer) — return 6:00 and 18:00 as sensible defaults
    return { sunrise: 360, sunset: 1080 };
  }

  const hourAngle = (Math.acos(cosHourAngle) * 180) / Math.PI;

  // Solar noon (local solar time, minutes from midnight)
  const solarNoon = 720 - 4 * lon;

  const sunrise = solarNoon - 4 * hourAngle;
  const sunset = solarNoon + 4 * hourAngle;

  return { sunrise, sunset };
}

/**
 * Determine which theme to use based on the current time + location.
 * Returns "dark" between sunset and sunrise, "light" otherwise.
 */
export function resolveAutoTheme(lat?: number, lon?: number): ResolvedTheme {
  const now = new Date();
  const location = lat != null && lon != null ? { lat, lon } : getSavedLocation();

  if (!location) {
    // No location saved — use a simple 6am-6pm rule
    const hour = now.getHours();
    return hour >= 6 && hour < 18 ? "light" : "dark";
  }

  const { sunrise, sunset } = calculateSunriseSunset(now, location.lat, location.lon);
  if (sunrise == null || sunset == null) {
    // Polar region — use the 6am-6pm fallback
    const hour = now.getHours();
    return hour >= 6 && hour < 18 ? "light" : "dark";
  }

  const nowMinutes = now.getHours() * 60 + now.getMinutes();
  // Dark between sunset and sunrise (next day)
  if (nowMinutes >= sunset || nowMinutes < sunrise) {
    return "dark";
  }
  return "light";
}

/**
 * Hook that returns the current resolved theme + a setter for the mode.
 * Automatically re-evaluates every 5 minutes (in case the user crosses
 * a sunrise/sunset boundary while the app is open).
 */
export function useTheme() {
  const [mode, setMode] = useState<ThemeMode>(getThemeMode());
  const [resolved, setResolved] = useState<ResolvedTheme>(() =>
    mode === "auto" ? resolveAutoTheme() : mode,
  );

  const evaluate = useCallback(() => {
    const m = getThemeMode();
    setMode(m);
    if (m === "auto") {
      setResolved(resolveAutoTheme());
    } else {
      setResolved(m);
    }
  }, []);

  useEffect(() => {
    evaluate();

    // Re-evaluate every 5 minutes (sunrise/sunset boundary crossing)
    const interval = setInterval(evaluate, 5 * 60 * 1000);

    // Listen for manual mode changes
    const onChange = () => evaluate();
    window.addEventListener("metardu-theme-change", onChange);

    return () => {
      clearInterval(interval);
      window.removeEventListener("metardu-theme-change", onChange);
    };
  }, [evaluate]);

  // Apply the theme to the document root
  useEffect(() => {
    document.documentElement.setAttribute("data-theme", resolved);
  }, [resolved]);

  const changeMode = useCallback((m: ThemeMode) => {
    setThemeMode(m);
    evaluate();
  }, [evaluate]);

  return { mode, resolved, setMode: changeMode };
}
