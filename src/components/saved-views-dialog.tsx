/**
 * Saved Views Dialog — Sprint 13.
 *
 * Save the current map view (extent + zoom + rotation + layers + domain)
 * under a name, and restore it later. Up to 20 saved views stored in
 * localStorage.
 *
 * Workflow:
 *   1. Position the map where you want it
 *   2. Open Saved Views (Ctrl+K → "saved views")
 *   3. Click "Save Current View" → enter name → save
 *   4. Later: open Saved Views → click a view → map restores
 */

import { useState } from "react";
import { Bookmark, Save, Trash2, MapPin, Clock } from "lucide-react";
import { colors } from "@/lib/tokens";
import { useSavedViewsStore, type SavedView } from "@/stores/saved-views-store";
import { useEscapeKey } from "@/lib/use-escape-key";
import { DialogShell, DialogButton, EmptyState } from "@/components/dialog-shell";

interface Props {
  open: boolean;
  onClose: () => void;
  /** Capture the current map state. Called when "Save Current View" is clicked. */
  onCapture: () => Omit<SavedView, "id" | "timestamp" | "name"> | null;
  /** Restore a saved view. Called when a view is clicked. */
  onRestore: (view: SavedView) => void;
}

export function SavedViewsDialog({ open, onClose, onCapture, onRestore }: Props) {
  const { views, save, remove, rename } = useSavedViewsStore();
  const [newName, setNewName] = useState("");
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editName, setEditName] = useState("");

  useEscapeKey(onClose, open);

  function handleSave() {
    const captured = onCapture();
    if (!captured || !newName.trim()) return;
    save({ ...captured, name: newName.trim() });
    setNewName("");
  }

  function handleRestore(view: SavedView) {
    onRestore(view);
    onClose();
  }

  return (
    <DialogShell
      open={open}
      onClose={onClose}
      title="Saved Views"
      icon={<Bookmark className="h-4 w-4" />}
      iconColor={colors.accent}
      maxWidth="max-w-xl"
      subtitle={`${views.length}/20 saved`}
      footerHint="Views store map extent + zoom + layers + domain"
      actions={<DialogButton variant="secondary" onClick={onClose}>Close</DialogButton>}
    >
      {/* Save current view */}
      <div className="mb-4 rounded-md border p-3" style={{ borderColor: `${colors.accent}40`, background: `${colors.accent}08` }}>
        <div className="mb-2 flex items-center gap-1.5 text-[10px] font-semibold uppercase tracking-wider" style={{ color: colors.accent }}>
          <Save className="h-3 w-3" /> Save Current View
        </div>
        <div className="flex gap-2">
          <input
            type="text"
            value={newName}
            onChange={(e) => setNewName(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && handleSave()}
            placeholder="e.g., Stockpile Pad A"
            className="flex-1 rounded-md border border-navy-border bg-navy-base px-3 py-1.5 text-sm text-white focus:border-accent focus:outline-none"
          />
          <DialogButton
            variant="primary"
            onClick={handleSave}
            disabled={!newName.trim()}
          >
            <Save className="h-3 w-3" /> Save
          </DialogButton>
        </div>
      </div>

      {/* Saved views list */}
      {views.length === 0 ? (
        <EmptyState
          icon={<Bookmark className="h-8 w-8" />}
          title="No saved views yet"
          description="Position the map where you want it, then enter a name above to save the current view."
        />
      ) : (
        <div className="space-y-1.5">
          {views.map((view) => (
            <div
              key={view.id}
              className="flex items-center gap-2 rounded-md border border-navy-border bg-navy-base p-2.5 hover:border-accent transition-colors group"
            >
              <MapPin className="h-4 w-4 flex-shrink-0" style={{ color: colors.accent }} />
              <div className="flex-1 min-w-0">
                {editingId === view.id ? (
                  <input
                    type="text"
                    value={editName}
                    onChange={(e) => setEditName(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter") {
                        rename(view.id, editName);
                        setEditingId(null);
                      } else if (e.key === "Escape") {
                        setEditingId(null);
                      }
                    }}
                    onBlur={() => {
                      rename(view.id, editName);
                      setEditingId(null);
                    }}
                    autoFocus
                    className="w-full rounded border border-accent bg-navy-base px-1.5 py-0.5 text-sm text-white focus:outline-none"
                  />
                ) : (
                  <div
                    className="text-sm font-medium text-white truncate cursor-pointer"
                    onClick={() => {
                      setEditingId(view.id);
                      setEditName(view.name);
                    }}
                    title="Click to rename"
                  >
                    {view.name}
                  </div>
                )}
                <div className="flex items-center gap-2 text-[10px] text-steel-gray">
                  <Clock className="h-2.5 w-2.5" />
                  {new Date(view.timestamp).toLocaleString()}
                  <span>·</span>
                  <span>{view.domain}</span>
                  <span>·</span>
                  <span>z={view.zoom.toFixed(1)}</span>
                </div>
              </div>
              <div className="flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                <DialogButton variant="marine" onClick={() => handleRestore(view)}>
                  Restore
                </DialogButton>
                <button
                  onClick={() => remove(view.id)}
                  className="rounded p-1.5 text-steel-gray hover:bg-fail/20 hover:text-fail"
                  title="Delete"
                  aria-label={`Delete ${view.name}`}
                >
                  <Trash2 className="h-3.5 w-3.5" />
                </button>
              </div>
            </div>
          ))}
        </div>
      )}
    </DialogShell>
  );
}
