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
import { IpcErrorToast } from "@/components/ipc-error-toast";
import { installIpcErrorReporter } from "@/lib/ipc-error-reporter";

// Install the global unhandled-rejection + window-error listener ONCE
// at module load. This must run before any Tauri `invoke()` call so
// we catch panics from the very first IPC round-trip.
installIpcErrorReporter();

function App() {
  const phase = useAppStore((s) => s.phase);
  const theme = useAppStore((s) => s.settings.theme);
  const density = useAppStore((s) => s.settings.density);
  const reducedMotion = useAppStore((s) => s.settings.reducedMotion);
  const hydrate = useAppStore((s) => s.hydrate);

  // One-shot: re-hydrate persisted settings + onboarding flag from
  // disk (Tauri) / localStorage (browser) BEFORE the boot sequence
  // decides whether to show onboarding or jump straight to workspace.
  // Without this, every cold boot would reset the user's saved
  // theme/CRS/density and re-show onboarding.
  useEffect(() => {
    void hydrate();
  }, [hydrate]);

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
        {/* IPC error banner — mounted at the root so it floats above
            every screen + dialog. Purely additive: any dialog that
            already handles its own try/catch will never see this. */}
        <IpcErrorToast />
        {phase === "splash" && <SplashScreen />}
        {phase === "modules" && <ModuleLoadingScreen />}
        {phase === "onboarding" && <OnboardingScreen />}
        {phase === "workspace" && <WorkspaceShell />}
      </div>
    </ErrorBoundary>
  );
}

export default App;
