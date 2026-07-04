import { useEscapeKey } from "@/lib/use-escape-key";
/**
 * Update Checker Dialog — Sprint 8.
 *
 * Checks for app updates and displays release notes. Lets the user
 * download + install updates when available.
 */

import { useState } from "react";
import {
  X, RefreshCw, Download, CheckCircle2, Loader2, Info,
} from "lucide-react";
import { colors } from "@/lib/tokens";
import {
  checkForUpdates, getCurrentVersion,
  type UpdateInfo,
} from "@/lib/tauri-ipc";

interface Props {
  open: boolean;
  onClose: () => void;
}

export function UpdateCheckerDialog({ open, onClose }: Props) {
  const [checking, setChecking] = useState(false);
  const [info, setInfo] = useState<UpdateInfo | null>(null);
  const [currentVer, setCurrentVer] = useState<string>("");
  const [error, setError] = useState<string | null>(null);

  useEscapeKey(onClose, open);
  if (!open) return null;

  async function handleCheck() {
    setChecking(true);
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
      } else {
        setError("Browser mode — update checking requires the native Tauri shell");
      }
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setChecking(false);
    }
  }

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
      onClick={onClose}
    >
      <div
        onClick={(e) => e.stopPropagation()}
        className="flex max-h-[88vh] w-full max-w-lg flex-col rounded-lg border border-navy-border bg-navy-panel shadow-2xl"
      >
        <div className="flex items-center justify-between border-b border-navy-border px-5 py-3">
          <h2 className="flex items-center gap-2 text-sm font-semibold text-white">
            <RefreshCw className="h-4 w-4" style={{ color: colors.industrialOrange }} />
            Check for Updates
          </h2>
          <button onClick={onClose} className="rounded p-1 text-steel-gray hover:bg-navy-elevated hover:text-white">
            <X className="h-4 w-4" />
          </button>
        </div>

        <div className="flex-1 overflow-y-auto p-5 space-y-4">
          {error && (
            <div className="rounded-md border p-3 text-xs"
              style={{ borderColor: `${colors.fail}40`, background: `${colors.fail}10`, color: colors.fail }}>
              {error}
            </div>
          )}

          <div className="rounded-md border border-navy-border bg-navy-base p-3 text-xs">
            <div className="flex items-center gap-2">
              <Info className="h-4 w-4" style={{ color: colors.marineTurquoise }} />
              <span className="text-steel-light">
                Current version: <span className="font-mono text-white">{currentVer || "—"}</span>
              </span>
            </div>
          </div>

          <button
            onClick={handleCheck}
            disabled={checking}
            className="flex items-center gap-2 rounded-md px-5 py-2 text-sm font-bold transition-colors disabled:opacity-40"
            style={{ background: colors.industrialOrange, color: colors.navyBase }}
          >
            {checking ? <Loader2 className="h-4 w-4 animate-spin" /> : <RefreshCw className="h-4 w-4" />}
            {checking ? "Checking…" : "Check for Updates"}
          </button>

          {info && (
            <div className="rounded-md border p-4"
              style={{
                borderColor: info.available ? `${colors.industrialOrange}40` : `${colors.pass}40`,
                background: info.available ? `${colors.industrialOrange}10` : `${colors.pass}10`,
              }}
            >
              {info.available ? (
                <>
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
                    <div className="mt-2 text-xs text-steel-light whitespace-pre-wrap">
                      {info.release_notes}
                    </div>
                  )}
                  {info.file_size > 0 && (
                    <div className="mt-2 text-[10px] text-steel-gray">
                      Size: {(info.file_size / 1024 / 1024).toFixed(1)} MB
                    </div>
                  )}
                  <button
                    className="mt-3 flex items-center gap-1 rounded-md px-3 py-1.5 text-xs font-medium"
                    style={{ background: colors.industrialOrange, color: colors.navyBase }}
                  >
                    <Download className="h-3 w-3" /> Download & Install
                  </button>
                </>
              ) : (
                <div className="flex items-center gap-2">
                  <CheckCircle2 className="h-5 w-5" style={{ color: colors.pass }} />
                  <div className="text-sm font-bold text-white">You're up to date!</div>
                </div>
              )}
            </div>
          )}
        </div>

        <div className="flex items-center justify-between border-t border-navy-border px-5 py-3">
          <span className="text-[10px] text-steel-gray">Updates are signature-verified before installation</span>
          <button
            onClick={onClose}
            className="rounded-md px-3 py-1 text-xs font-medium"
            style={{ background: colors.pass, color: colors.navyBase }}
          >
            Close
          </button>
        </div>
      </div>
    </div>
  );
}
