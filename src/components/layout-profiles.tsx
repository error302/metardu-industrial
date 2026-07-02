/**
 * Layout Profiles — Sprint 5 Priority #7.
 *
 * Predefined panel arrangements for common workflows. One-click switch
 * rearranges sidebar + right panel + map-only states for survey control
 * rooms with multi-monitor setups.
 *
 * Three presets:
 *   - DEFAULT — sidebar + map + right panel (current state)
 *   - DATA_INGEST — sidebar (data sources) + map (maximal space)
 *   - BATHYMETRY_CLEAN — map-only (max canvas for QC)
 *   - VOLUME_REPORTING — sidebar + map + right panel (statistics)
 *
 * State preserved per layout in localStorage so switching back restores.
 */

import { useEffect } from "react";
import { LayoutGrid, Database, Waves, Calculator } from "lucide-react";
import { colors } from "@/lib/tokens";

export type LayoutProfile = "default" | "data_ingest" | "bathymetry_clean" | "volume_reporting";

interface Props {
  active: LayoutProfile;
  onChange: (layout: LayoutProfile) => void;
}

interface ProfileDef {
  id: LayoutProfile;
  label: string;
  description: string;
  icon: React.ReactNode;
  // Settings applied when this layout is active
  sidebarOpen: boolean;
  rightPanelOpen: boolean;
}

const PROFILES: ProfileDef[] = [
  {
    id: "default",
    label: "Default",
    description: "Sidebar + map + right panel",
    icon: <LayoutGrid className="h-3.5 w-3.5" />,
    sidebarOpen: true,
    rightPanelOpen: true,
  },
  {
    id: "data_ingest",
    label: "Data Ingest",
    description: "Sidebar expanded, map maximized",
    icon: <Database className="h-3.5 w-3.5" />,
    sidebarOpen: true,
    rightPanelOpen: false,
  },
  {
    id: "bathymetry_clean",
    label: "Bathy Clean",
    description: "Map-only — maximal canvas for QC",
    icon: <Waves className="h-3.5 w-3.5" />,
    sidebarOpen: false,
    rightPanelOpen: false,
  },
  {
    id: "volume_reporting",
    label: "Volume Report",
    description: "Sidebar + map + right panel (stats)",
    icon: <Calculator className="h-3.5 w-3.5" />,
    sidebarOpen: true,
    rightPanelOpen: true,
  },
];

export function LayoutProfiles({ active, onChange }: Props) {
  // Persist the active layout in localStorage
  useEffect(() => {
    try {
      localStorage.setItem("metardu.layout", active);
    } catch {
      // localStorage may be unavailable (private mode, sandbox)
    }
  }, [active]);

  return (
    <div className="flex items-center gap-0.5 rounded-md border border-navy-border bg-navy-base/80 p-0.5">
      {PROFILES.map((p) => {
        const isActive = active === p.id;
        return (
          <button
            key={p.id}
            onClick={() => onChange(p.id)}
            title={p.description}
            className="flex items-center gap-1 rounded px-2 py-1 text-[10px] font-medium transition-colors"
            style={{
              background: isActive ? colors.industrialOrange : "transparent",
              color: isActive ? colors.navyBase : colors.steelLight,
            }}
          >
            {p.icon}
            <span className="hidden lg:inline">{p.label}</span>
          </button>
        );
      })}
    </div>
  );
}

/** Get the panel-open settings for a given layout. */
export function getLayoutSettings(layout: LayoutProfile): { sidebarOpen: boolean; rightPanelOpen: boolean } {
  const profile = PROFILES.find((p) => p.id === layout) ?? PROFILES[0];
  return {
    sidebarOpen: profile.sidebarOpen,
    rightPanelOpen: profile.rightPanelOpen,
  };
}

/** Load the persisted layout from localStorage. */
export function loadPersistedLayout(): LayoutProfile {
  try {
    const stored = localStorage.getItem("metardu.layout") as LayoutProfile | null;
    if (stored && PROFILES.some((p) => p.id === stored)) {
      return stored;
    }
  } catch {
    // localStorage unavailable
  }
  return "default";
}
