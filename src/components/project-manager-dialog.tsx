/**
 * Project Manager Dialog — Sprint 8 (extended Sprint 11).
 *
 * Save/load/manage .metardu project files. Shows recent projects,
 * current project state, save/load actions, and a template picker
 * for new projects (Sprint 11).
 */

import { useState } from "react";
import {
  Save, FolderOpen, FilePlus, Clock, Loader2, LayoutTemplate,
} from "lucide-react";
import { colors } from "@/lib/tokens";
import { DialogShell, DialogButton } from "@/components/dialog-shell";
import {
  newProject, saveProject, loadProject,
  type MetarduProject,
} from "@/lib/tauri-ipc";
import {
  PROJECT_TEMPLATES, dialogLabels,
  type ProjectTemplate, type DialogKey,
} from "@/lib/project-templates";

interface Props {
  open: boolean;
  onClose: () => void;
  currentProject: MetarduProject | null;
  onProjectLoaded: (project: MetarduProject) => void;
  /** Called when a template is applied — opens the listed dialogs. */
  onOpenDialogs?: (dialogs: DialogKey[]) => void;
}

export function ProjectManagerDialog({ open, onClose, currentProject, onProjectLoaded, onOpenDialogs }: Props) {
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
  const [selectedTemplate, setSelectedTemplate] = useState<ProjectTemplate>(PROJECT_TEMPLATES[0]);


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
        // Apply template — auto-open the listed dialogs
        if (selectedTemplate.dialogsToOpen.length > 0 && onOpenDialogs) {
          onOpenDialogs(selectedTemplate.dialogsToOpen);
        }
      } else {
        setError("Browser mode — project creation requires the native Tauri shell");
      }
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setCreating(false);
    }
  }

  /** Apply a template's defaults to the form fields. */
  function applyTemplate(t: ProjectTemplate) {
    setSelectedTemplate(t);
    setNewEpsg(t.defaultEpsg);
    setNewDomain(t.domain);
    if (t.id !== "blank") {
      setNewName(`${t.namePrefix}-${new Date().toISOString().slice(0, 10)}`);
    }
  }

return (
    <DialogShell
      open={open}
      onClose={onClose}
      title="Project Manager"
      icon={<FolderOpen className="h-4 w-4" />}
      iconColor={colors.industrialOrange}
      maxWidth="max-w-2xl"
      subtitle="Save/load .metardu files"
      footerHint="Auto-save + versioning"
      actions={
        <DialogButton variant="secondary" onClick={onClose}>Close</DialogButton>
      }
    >
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
            <div className="input-enterprise rounded-md border border-navy-border bg-navy-base p-3">
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
            <h3 className="flex items-center gap-1 text-[11px] font-semibold uppercase tracking-wider text-steel-light">
              <LayoutTemplate className="h-3 w-3" /> New Project from Template
            </h3>
            {/* Template grid */}
            <div className="grid grid-cols-2 gap-2 mb-3">
              {PROJECT_TEMPLATES.map(t => (
                <button
                  key={t.id}
                  onClick={() => applyTemplate(t)}
                  className="text-left rounded-md border p-2 transition-colors"
                  style={{
                    borderColor: selectedTemplate.id === t.id ? colors.industrialOrange : colors.navyBorder,
                    background: selectedTemplate.id === t.id ? `${colors.industrialOrange}10` : colors.navyBase,
                  }}
                >
                  <div className="flex items-center gap-1.5">
                    <span className="text-base" style={{ color: colors.industrialOrange }}>{t.icon}</span>
                    <span className="text-xs font-semibold text-white">{t.name}</span>
                  </div>
                  <div className="mt-0.5 text-[10px] leading-tight text-steel-gray">{t.description}</div>
                  {t.dialogsToOpen.length > 0 && (
                    <div className="mt-1 text-[9px] text-steel-gray">
                      <span className="text-steel-light">Opens:</span> {dialogLabels(t.dialogsToOpen).join(", ")}
                    </div>
                  )}
                </button>
              ))}
            </div>
            <div className="grid grid-cols-3 gap-2">
              <input
                type="text" value={newName} onChange={(e) => setNewName(e.target.value)}
                placeholder="Project name"
                className="input-enterprise rounded-md border border-navy-border bg-navy-base px-2 py-1.5 text-xs text-white focus:outline-none"
              />
              <input
                type="text" value={newEpsg} onChange={(e) => setNewEpsg(e.target.value)}
                placeholder="EPSG:4326"
                className="input-enterprise rounded-md border border-navy-border bg-navy-base px-2 py-1.5 font-mono text-xs text-white focus:outline-none"
              />
              <select
                value={newDomain} onChange={(e) => setNewDomain(e.target.value)}
                className="input-enterprise rounded-md border border-navy-border bg-navy-base px-2 py-1.5 text-xs text-white focus:outline-none"
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
              Create from "{selectedTemplate.name}"
            </button>
          </div>

          {/* Save */}
          <div className="space-y-2">
            <h3 className="text-[11px] font-semibold uppercase tracking-wider text-steel-light">Save Project</h3>
            <div className="flex gap-2">
              <input
                type="text" value={savePath} onChange={(e) => setSavePath(e.target.value)}
                placeholder="/path/to/project.metardu"
                className="input-enterprise flex-1 rounded-md border border-navy-border bg-navy-base px-2 py-1.5 font-mono text-xs text-white focus:outline-none"
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
                className="input-enterprise flex-1 rounded-md border border-navy-border bg-navy-base px-2 py-1.5 font-mono text-xs text-white focus:outline-none"
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
                  <div key={i} className="input-enterprise rounded border border-navy-border bg-navy-base px-2 py-1 font-mono text-[10px] text-steel-light truncate">
                    {r}
                  </div>
                ))}
              </div>
            </div>
          )}
    </DialogShell>
  );
}
