/**
 * MetaRDU Industrial — Application Root
 *
 * Boot sequence per ARCHITECTURE.md §7:
 *   splash → modules → onboarding (first run) → workspace
 *
 * Once onboarding is complete, subsequent boots skip onboarding and
 * go straight from modules → workspace.
 */

import { useAppStore } from "@/stores/app-store";
import { SplashScreen } from "@/screens/splash-screen";
import { ModuleLoadingScreen } from "@/screens/module-loading-screen";
import { OnboardingScreen } from "@/screens/onboarding-screen";
import { WorkspaceShell } from "@/screens/workspace-shell";

function App() {
  const phase = useAppStore((s) => s.phase);

  return (
    <div className="h-screen w-screen overflow-hidden bg-navy-base">
      {phase === "splash" && <SplashScreen />}
      {phase === "modules" && <ModuleLoadingScreen />}
      {phase === "onboarding" && <OnboardingScreen />}
      {phase === "workspace" && <WorkspaceShell />}
    </div>
  );
}

export default App;
