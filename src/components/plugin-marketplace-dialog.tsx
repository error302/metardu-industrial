import { useEscapeKey } from "@/lib/use-escape-key";
/**
 * Plugin Marketplace Dialog — Sprint 8.
 *
 * Browse, search, install, and uninstall third-party plugins from
 * a plugin registry. Shows installed plugins + available plugins.
 */

import { useState, useEffect } from "react";
import {
  X, Search, Download, Trash2, CheckCircle2, Loader2, Package, Shield,
} from "lucide-react";
import { colors } from "@/lib/tokens";
import {
  fetchPluginRegistry, listInstalledPlugins, installPlugin, uninstallPlugin,
  type PluginRegistry, type RegistryPlugin, type InstalledPlugin,
} from "@/lib/tauri-ipc";

interface Props {
  open: boolean;
  onClose: () => void;
}

export function PluginMarketplaceDialog({ open, onClose }: Props) {
  const [registry, setRegistry] = useState<PluginRegistry | null>(null);
  const [installed, setInstalled] = useState<InstalledPlugin[]>([]);
  const [query, setQuery] = useState("");
  const [loading, setLoading] = useState(false);
  const [installing, setInstalling] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [registrySource, setRegistrySource] = useState("");

  useEffect(() => {
    if (open) {
      refreshInstalled();
    }
  }, [open]);

  useEscapeKey(onClose, open);
  if (!open) return null;

  async function refreshInstalled() {
    try {
      const list = await listInstalledPlugins();
      setInstalled(list);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  async function handleFetchRegistry() {
    setLoading(true);
    setError(null);
    try {
      const r = await fetchPluginRegistry(registrySource);
      if (r) {
        setRegistry(r);
      } else {
        setError("Browser mode — registry fetch requires the native Tauri shell");
      }
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }

  async function handleInstall(plugin: RegistryPlugin) {
    if (!registry) return;
    setInstalling(plugin.id);
    setError(null);
    try {
      await installPlugin(registry, plugin.id);
      await refreshInstalled();
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setInstalling(null);
    }
  }

  async function handleUninstall(pluginId: string) {
    setError(null);
    try {
      await uninstallPlugin(pluginId);
      await refreshInstalled();
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  const filteredPlugins = registry?.plugins.filter((p) => {
    if (!query) return true;
    const q = query.toLowerCase();
    return p.name.toLowerCase().includes(q) ||
           p.description.toLowerCase().includes(q) ||
           p.vendor.toLowerCase().includes(q) ||
           p.id.toLowerCase().includes(q);
  }) ?? [];

  const isInstalled = (id: string) => installed.some((p) => p.id === id);

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
      onClick={onClose}
    >
      <div
        onClick={(e) => e.stopPropagation()}
        className="flex max-h-[88vh] w-full max-w-3xl flex-col rounded-lg border border-navy-border bg-navy-panel shadow-2xl"
      >
        <div className="flex items-center justify-between border-b border-navy-border px-5 py-3">
          <h2 className="flex items-center gap-2 text-sm font-semibold text-white">
            <Package className="h-4 w-4" style={{ color: colors.industrialOrange }} />
            Plugin Marketplace
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

          {/* Installed plugins */}
          {installed.length > 0 && (
            <div>
              <h3 className="mb-2 flex items-center gap-1 text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
                <CheckCircle2 className="h-3 w-3" style={{ color: colors.pass }} />
                Installed ({installed.length})
              </h3>
              <div className="space-y-1">
                {installed.map((p) => (
                  <div key={p.id} className="flex items-center gap-2 rounded-md border border-navy-border bg-navy-base p-2">
                    <div className="flex-1 min-w-0">
                      <div className="text-xs font-medium text-white">{p.name}</div>
                      <div className="text-[10px] text-steel-gray">
                        v{p.version} · {p.vendor} · installed {p.installed_date}
                      </div>
                    </div>
                    <button
                      onClick={() => handleUninstall(p.id)}
                      className="rounded p-1 text-steel-gray hover:text-fail"
                    >
                      <Trash2 className="h-3.5 w-3.5" />
                    </button>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Registry source */}
          <div>
            <h3 className="mb-2 text-[10px] font-semibold uppercase tracking-wider text-steel-gray">Plugin Registry</h3>
            <div className="flex gap-2">
              <input
                type="text" value={registrySource} onChange={(e) => setRegistrySource(e.target.value)}
                placeholder="https://registry.metardu.example/plugins.json or /path/to/registry.json"
                className="flex-1 rounded-md border border-navy-border bg-navy-base px-2 py-1.5 font-mono text-xs text-white focus:outline-none"
              />
              <button
                onClick={handleFetchRegistry} disabled={loading || !registrySource}
                className="flex items-center gap-1 rounded-md px-3 py-1.5 text-xs font-medium disabled:opacity-40"
                style={{ background: colors.marineTurquoise, color: colors.navyBase }}
              >
                {loading ? <Loader2 className="h-3 w-3 animate-spin" /> : <Search className="h-3 w-3" />}
                Fetch
              </button>
            </div>
          </div>

          {/* Search + plugin list */}
          {registry && (
            <div>
              <div className="mb-2 flex items-center gap-2">
                <Search className="h-3.5 w-3.5 text-steel-gray" />
                <input
                  type="text" value={query} onChange={(e) => setQuery(e.target.value)}
                  placeholder="Search plugins…"
                  className="flex-1 rounded-md border border-navy-border bg-navy-base px-2 py-1.5 text-xs text-white focus:outline-none"
                />
                <span className="text-[10px] text-steel-gray">{filteredPlugins.length} plugins</span>
              </div>

              <div className="space-y-2">
                {filteredPlugins.map((plugin) => {
                  const installedAlready = isInstalled(plugin.id);
                  return (
                    <div key={plugin.id} className="rounded-md border border-navy-border bg-navy-base p-3">
                      <div className="flex items-start justify-between gap-2">
                        <div className="flex-1 min-w-0">
                          <div className="flex items-center gap-2">
                            <span className="text-xs font-bold text-white">{plugin.name}</span>
                            {plugin.official && (
                              <span className="rounded px-1 py-0.5 text-[8px] font-bold uppercase"
                                style={{ background: `${colors.industrialOrange}20`, color: colors.industrialOrange }}>
                                Official
                              </span>
                            )}
                            <span className="text-[10px] text-steel-gray">v{plugin.version}</span>
                          </div>
                          <div className="text-[10px] text-steel-gray mt-0.5">by {plugin.vendor}</div>
                          <div className="text-[10px] text-steel-light mt-1">{plugin.description}</div>
                          <div className="flex items-center gap-2 mt-1 text-[9px] text-steel-gray">
                            <span>{plugin.downloads.toLocaleString()} downloads</span>
                            <span>·</span>
                            <span>{plugin.license}</span>
                            {plugin.extensions.length > 0 && (
                              <>
                                <span>·</span>
                                <span>.{plugin.extensions.join(", .")}</span>
                              </>
                            )}
                          </div>
                        </div>
                        {installedAlready ? (
                          <span className="flex items-center gap-1 text-[10px]" style={{ color: colors.pass }}>
                            <CheckCircle2 className="h-3 w-3" /> Installed
                          </span>
                        ) : (
                          <button
                            onClick={() => handleInstall(plugin)}
                            disabled={installing === plugin.id}
                            className="flex items-center gap-1 rounded-md px-2 py-1 text-[10px] font-medium disabled:opacity-40"
                            style={{ background: colors.industrialOrange, color: colors.navyBase }}
                          >
                            {installing === plugin.id ? <Loader2 className="h-3 w-3 animate-spin" /> : <Download className="h-3 w-3" />}
                            Install
                          </button>
                        )}
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>
          )}
        </div>

        <div className="flex items-center justify-between border-t border-navy-border px-5 py-3">
          <span className="flex items-center gap-1 text-[10px] text-steel-gray">
            <Shield className="h-3 w-3" /> All plugins are SHA-256 verified before installation
          </span>
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
