/**
 * Update Checker Dialog — Production Edition.
 *
 * Uses tauri-plugin-updater for real signed updates:
 *   1. User clicks "Check for Updates" → check_for_updates_cmd
 *   2. If available, shows release notes + "Download & Install" button
 *   3. User clicks "Download & Install" → download_and_install_update_cmd
 *   4. Plugin downloads bundle, verifies Ed25519 signature, installs
 *   5. Shows "Restart to apply update" prompt
 *
 * If the updater is not configured (no pubkey/endpoints in
 * tauri.conf.json), shows a helpful message instead of crashing.
 */

import { useState } from "react";
import {
  RefreshCw, Download, CheckCircle2, Loader2, Info, AlertCircle, RotateCw,
} from "lucide-react";
import { colors } from "@/lib/tokens";
import { DialogShell, DialogButton } from "@/components/dialog-shell";
import {
  checkForUpdates, getCurrentVersion, downloadAndInstallUpdate,
  type UpdateInfo,
} from "@/lib/tauri-ipc";

interface Props {
  open: boolean;
  onClose: () => void;
}

type Phase = "idle" | "checking" | "available" | "uptodate" | "downloading" | "installed" | "error" | "notconfigured";

export function UpdateCheckerDialog({ open, onClose }: Props) {
  const [phase, setPhase] = useState<Phase>("idle");
  const [info, setInfo] = useState<UpdateInfo | null>(null);
  const [currentVer, setCurrentVer] = useState<string>("");
  const [error, setError] = useState<string | null>(null);


  async function handleCheck() {
    setPhase("checking");
    setError(null);
    setInfo(null);
    try {
      const [ver, updateInfo] = await Promise.all([
        getCurrentVersion(),
        checkForUpdates(),
      ]);
      setCurrentVer(ver);
      if (updateInfo) {
        setInfo(updateInfo);
        setPhase(updateInfo.available ? "available" : "uptodate");
      } else {
        setError("Browser mode — update checking requires the native Tauri shell");
        setPhase("error");
      }
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      if (msg.includes("not configured") || msg.includes("NotConfigured")) {
        setPhase("notconfigured");
      } else {
        setError(msg);
        setPhase("error");
      }
    }
  }

  async function handleDownloadAndInstall() {
    setPhase("downloading");
    setError(null);
    try {
      await downloadAndInstallUpdate();
      setPhase("installed");
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
      setPhase("error");
    }
  }

  function handleRestart() {
    // The Tauri updater plugin handles restart automatically on
    // next launch. We just close the dialog — the user can restart
    // at their convenience. The installed update will apply on
    // next app start.
    onClose();
    // Optionally, we could call relaunch() here:
    // import { relaunch } from "@tauri-apps/plugin-process";
    // void relaunch();
  }

return (
    <DialogShell
      open={open}
      onClose={onClose}
      title="Check for Updates"
      icon={<RefreshCw className="h-4 w-4" />}
      iconColor={colors.steelLight}
      maxWidth="max-w-lg"
      subtitle="Signed auto-updater"
      footerHint="RSA-PSS packages"
      actions={
        <DialogButton variant="secondary" onClick={onClose}>Close</DialogButton>
      }
    >
          {/* Error */}
          {phase === "error" && error && (
            <div className="rounded-md border p-3 text-xs"
              style={{ borderColor: `${colors.fail}40`, background: `${colors.fail}10`, color: colors.fail }}>
              {error}
            </div>
          )}

          {/* Not configured */}
          {phase === "notconfigured" && (
            <div className="rounded-md border p-4 text-xs"
              style={{ borderColor: `${colors.investigate}40`, background: `${colors.investigate}10` }}>
              <div className="flex items-center gap-2 mb-2 font-semibold" style={{ color: colors.investigate }}>
                <AlertCircle className="h-4 w-4" />
                Auto-update not configured
              </div>
              <div className="text-steel-light">
                The auto-updater requires a signing public key and update endpoint
                to be configured in <code className="text-white">tauri.conf.json</code>.
                See <code className="text-white">RELEASE.md</code> for setup instructions.
                You can still check for updates manually on the
                <a href="https://github.com/error302/metardu-industrial/releases"
                   target="_blank" rel="noopener noreferrer"
                   className="underline ml-1" style={{ color: colors.industrialOrange }}>
                  GitHub releases page
                </a>.
              </div>
            </div>
          )}

          {/* Current version info */}
          <div className="rounded-md border border-navy-border bg-navy-base p-3 text-xs">
            <div className="flex items-center gap-2">
              <Info className="h-4 w-4" style={{ color: colors.marineTurquoise }} />
              <span className="text-steel-light">
                Current version: <span className="font-mono text-white">{currentVer || "—"}</span>
              </span>
            </div>
          </div>

          {/* Check button — show when idle, error, or not configured */}
          {(phase === "idle" || phase === "error" || phase === "notconfigured") && (
            <button
              onClick={handleCheck}
              className="flex items-center gap-2 rounded-md px-5 py-2 text-sm font-bold transition-colors"
              style={{ background: colors.industrialOrange, color: colors.navyBase }}
            >
              <RefreshCw className="h-4 w-4" />
              Check for Updates
            </button>
          )}

          {/* Update available */}
          {phase === "available" && info && (
            <div className="rounded-md border p-4"
              style={{
                borderColor: `${colors.industrialOrange}40`,
                background: `${colors.industrialOrange}10`,
              }}>
              <div className="flex items-center gap-2 mb-2">
                <Download className="h-5 w-5" style={{ color: colors.industrialOrange }} />
                <div>
                  <div className="text-sm font-bold text-white">Update Available!</div>
                  <div className="text-[10px] text-steel-gray">
                    v{currentVer} → v{info.latest_version} · {info.release_date}
                  </div>
                </div>
              </div>
              {info.release_notes && (
                <div className="mt-2 text-xs text-steel-light whitespace-pre-wrap max-h-40 overflow-y-auto">
                  {info.release_notes}
                </div>
              )}
              {info.file_size > 0 && (
                <div className="mt-2 text-[10px] text-steel-gray">
                  Size: {(info.file_size / 1024 / 1024).toFixed(1)} MB
                </div>
              )}
              <button
                onClick={handleDownloadAndInstall}
                className="mt-3 flex items-center gap-1 rounded-md px-3 py-1.5 text-xs font-medium"
                style={{ background: colors.industrialOrange, color: colors.navyBase }}
              >
                <Download className="h-3 w-3" /> Download & Install
              </button>
            </div>
          )}

          {/* Downloading */}
          {phase === "downloading" && (
            <div className="rounded-md border p-4"
              style={{
                borderColor: `${colors.marineTurquoise}40`,
                background: `${colors.marineTurquoise}10`,
              }}>
              <div className="flex items-center gap-2 mb-3">
                <Loader2 className="h-5 w-5 animate-spin" style={{ color: colors.marineTurquoise }} />
                <div className="text-sm font-bold text-white">Downloading update…</div>
              </div>
              <div className="text-xs text-steel-light">
                The update is being downloaded and signature-verified.
                This may take a few minutes depending on your connection.
              </div>
            </div>
          )}

          {/* Installed — restart required */}
          {phase === "installed" && (
            <div className="rounded-md border p-4"
              style={{
                borderColor: `${colors.pass}40`,
                background: `${colors.pass}10`,
              }}>
              <div className="flex items-center gap-2 mb-2">
                <CheckCircle2 className="h-5 w-5" style={{ color: colors.pass }} />
                <div className="text-sm font-bold text-white">Update installed!</div>
              </div>
              <div className="text-xs text-steel-light mb-3">
                Restart MetaRDU Industrial to apply the update.
              </div>
              <button
                onClick={handleRestart}
                className="flex items-center gap-1 rounded-md px-3 py-1.5 text-xs font-medium"
                style={{ background: colors.pass, color: colors.navyBase }}
              >
                <RotateCw className="h-3 w-3" /> Restart to apply
              </button>
            </div>
          )}

          {/* Up to date */}
          {phase === "uptodate" && (
            <div className="rounded-md border p-4"
              style={{
                borderColor: `${colors.pass}40`,
                background: `${colors.pass}10`,
              }}>
              <div className="flex items-center gap-2">
                <CheckCircle2 className="h-5 w-5" style={{ color: colors.pass }} />
                <div>
                  <div className="text-sm font-bold text-white">You're up to date!</div>
                  <div className="text-[10px] text-steel-gray">
                    Version {info?.latest_version || currentVer}
                  </div>
                </div>
              </div>
            </div>
          )}
    </DialogShell>
  );
}
