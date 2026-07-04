/**
 * MetaRDU Industrial — Application Root
 *
 * Boot sequence per ARCHITECTURE.md §7:
 *   splash → modules → onboarding (first run) → workspace
 *
 * Once onboarding is complete, subsequent boots skip onboarding and
 * go straight from modules → workspace.
 */

import { useEffect } from "react";
import { useAppStore } from "@/stores/app-store";
import { SplashScreen } from "@/screens/splash-screen";
import { ModuleLoadingScreen } from "@/screens/module-loading-screen";
import { OnboardingScreen } from "@/screens/onboarding-screen";
import { WorkspaceShell } from "@/screens/workspace-shell";
import { ErrorBoundary } from "@/components/error-boundary";

function App() {
  const phase = useAppStore((s) => s.phase);
  const theme = useAppStore((s) => s.settings.theme);
  const density = useAppStore((s) => s.settings.density);
  const reducedMotion = useAppStore((s) => s.settings.reducedMotion);

  useEffect(() => {
    document.documentElement.setAttribute("data-theme", theme);
  }, [theme]);

  useEffect(() => {
    document.documentElement.setAttribute("data-density", density);
  }, [density]);

  useEffect(() => {
    document.documentElement.setAttribute("data-reduced-motion", reducedMotion ? "true" : "false");
  }, [reducedMotion]);

  return (
    <ErrorBoundary>
      <div className="h-screen w-screen overflow-hidden bg-navy-base">
        {phase === "splash" && <SplashScreen />}
        {phase === "modules" && <ModuleLoadingScreen />}
        {phase === "onboarding" && <OnboardingScreen />}
        {phase === "workspace" && <WorkspaceShell />}
      </div>
    </ErrorBoundary>
  );
}

export default App;
