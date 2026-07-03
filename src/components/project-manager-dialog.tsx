/**
 * Project Manager Dialog — Sprint 8.
 *
 * Save/load/manage .metardu project files. Shows recent projects,
 * current project state, and save/load actions.
 */

import { useState } from "react";
import {
  X, Save, FolderOpen, FilePlus, Clock, FileBox, Loader2,
} from "lucide-react";
import { colors } from "@/lib/tokens";
import {
  newProject, saveProject, loadProject,
  type MetarduProject,
} from "@/lib/tauri-ipc";

interface Props {
  open: boolean;
  onClose: () => void;
  currentProject: MetarduProject | null;
  onProjectLoaded: (project: MetarduProject) => void;
}

export function ProjectManagerDialog({ open, onClose, currentProject, onProjectLoaded }: Props) {
  const [saving, setSaving] = useState(false);
  const [loading, setLoading] = useState(false);
  const [creating, setCreating] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const [savePath, setSavePath] = useState("/tmp/metardu_project.metardu");
  const [loadPath, setLoadPath] = useState("");
  const [newName, setNewName] = useState("Untitled Project");
  const [newEpsg, setNewEpsg] = useState("EPSG:4326");
  const [newDomain, setNewDomain] = useState("both");

  if (!open) return null;

  async function handleSave() {
    if (!currentProject) return;
    setSaving(true);
    setError(null);
    setSuccess(null);
    try {
      const result = await saveProject(currentProject, savePath);
      if (result) {
        setSuccess(`Project saved to ${result}`);
      } else {
        setError("Browser mode — saving requires the native Tauri shell");
      }
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setSaving(false);
    }
  }

  async function handleLoad() {
    setLoading(true);
    setError(null);
    setSuccess(null);
    try {
      const project = await loadProject(loadPath);
      if (project) {
        onProjectLoaded(project);
        setSuccess(`Loaded project: ${project.name}`);
      } else {
        setError("Browser mode — loading requires the native Tauri shell");
      }
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }

  async function handleNew() {
    setCreating(true);
    setError(null);
    setSuccess(null);
    try {
      const project = await newProject({
        name: newName,
        defaultEpsg: newEpsg,
        domain: newDomain,
      });
      if (project) {
        onProjectLoaded(project);
        setSuccess(`Created new project: ${project.name}`);
      } else {
        setError("Browser mode — project creation requires the native Tauri shell");
      }
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setCreating(false);
    }
  }

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
      onClick={onClose}
    >
      <div
        onClick={(e) => e.stopPropagation()}
        className="flex max-h-[88vh] w-full max-w-2xl flex-col rounded-lg border border-navy-border bg-navy-panel shadow-2xl"
      >
        <div className="flex items-center justify-between border-b border-navy-border px-5 py-3">
          <h2 className="flex items-center gap-2 text-sm font-semibold text-white">
            <FileBox className="h-4 w-4" style={{ color: colors.industrialOrange }} />
            Project Manager
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
          {success && (
            <div className="rounded-md border p-3 text-xs"
              style={{ borderColor: `${colors.pass}40`, background: `${colors.pass}10`, color: colors.pass }}>
              {success}
            </div>
          )}

          {/* Current project status */}
          {currentProject && (
            <div className="rounded-md border border-navy-border bg-navy-base p-3">
              <div className="text-[10px] uppercase tracking-wider text-steel-gray">Current Project</div>
              <div className="text-sm font-bold text-white">{currentProject.name}</div>
              <div className="text-[10px] text-steel-gray mt-1">
                {currentProject.files.length} files · EPSG:{currentProject.default_epsg} · {currentProject.domain}
              </div>
              <div className="text-[10px] text-steel-gray">
                Created: {currentProject.created} · Modified: {currentProject.modified}
              </div>
            </div>
          )}

          {/* New project */}
          <div className="space-y-2">
            <h3 className="text-[11px] font-semibold uppercase tracking-wider text-steel-light">New Project</h3>
            <div className="grid grid-cols-3 gap-2">
              <input
                type="text" value={newName} onChange={(e) => setNewName(e.target.value)}
                placeholder="Project name"
                className="rounded-md border border-navy-border bg-navy-base px-2 py-1.5 text-xs text-white focus:outline-none"
              />
              <input
                type="text" value={newEpsg} onChange={(e) => setNewEpsg(e.target.value)}
                placeholder="EPSG:4326"
                className="rounded-md border border-navy-border bg-navy-base px-2 py-1.5 font-mono text-xs text-white focus:outline-none"
              />
              <select
                value={newDomain} onChange={(e) => setNewDomain(e.target.value)}
                className="rounded-md border border-navy-border bg-navy-base px-2 py-1.5 text-xs text-white focus:outline-none"
              >
                <option value="both">Both</option>
                <option value="mining">Mining</option>
                <option value="marine">Marine</option>
              </select>
            </div>
            <button
              onClick={handleNew} disabled={creating}
              className="flex items-center gap-1 rounded-md px-3 py-1.5 text-xs font-medium disabled:opacity-40"
              style={{ background: colors.industrialOrange, color: colors.navyBase }}
            >
              {creating ? <Loader2 className="h-3 w-3 animate-spin" /> : <FilePlus className="h-3 w-3" />}
              Create New Project
            </button>
          </div>

          {/* Save */}
          <div className="space-y-2">
            <h3 className="text-[11px] font-semibold uppercase tracking-wider text-steel-light">Save Project</h3>
            <div className="flex gap-2">
              <input
                type="text" value={savePath} onChange={(e) => setSavePath(e.target.value)}
                placeholder="/path/to/project.metardu"
                className="flex-1 rounded-md border border-navy-border bg-navy-base px-2 py-1.5 font-mono text-xs text-white focus:outline-none"
              />
              <button
                onClick={handleSave} disabled={saving || !currentProject}
                className="flex items-center gap-1 rounded-md px-3 py-1.5 text-xs font-medium disabled:opacity-40"
                style={{ background: colors.pass, color: colors.navyBase }}
              >
                {saving ? <Loader2 className="h-3 w-3 animate-spin" /> : <Save className="h-3 w-3" />}
                Save
              </button>
            </div>
          </div>

          {/* Load */}
          <div className="space-y-2">
            <h3 className="text-[11px] font-semibold uppercase tracking-wider text-steel-light">Open Project</h3>
            <div className="flex gap-2">
              <input
                type="text" value={loadPath} onChange={(e) => setLoadPath(e.target.value)}
                placeholder="/path/to/project.metardu"
                className="flex-1 rounded-md border border-navy-border bg-navy-base px-2 py-1.5 font-mono text-xs text-white focus:outline-none"
              />
              <button
                onClick={handleLoad} disabled={loading || !loadPath}
                className="flex items-center gap-1 rounded-md px-3 py-1.5 text-xs font-medium disabled:opacity-40"
                style={{ background: colors.marineTurquoise, color: colors.navyBase }}
              >
                {loading ? <Loader2 className="h-3 w-3 animate-spin" /> : <FolderOpen className="h-3 w-3" />}
                Open
              </button>
            </div>
          </div>

          {currentProject?.recent_reports && currentProject.recent_reports.length > 0 && (
            <div className="space-y-2">
              <h3 className="flex items-center gap-1 text-[11px] font-semibold uppercase tracking-wider text-steel-light">
                <Clock className="h-3 w-3" /> Recent Reports
              </h3>
              <div className="space-y-1">
                {currentProject.recent_reports.slice(0, 5).map((r, i) => (
                  <div key={i} className="rounded border border-navy-border bg-navy-base px-2 py-1 font-mono text-[10px] text-steel-light truncate">
                    {r}
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>

        <div className="flex items-center justify-between border-t border-navy-border px-5 py-3">
          <span className="text-[10px] text-steel-gray">Projects save as .metardu JSON files</span>
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
