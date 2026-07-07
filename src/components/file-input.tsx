/**
 * FileInput — reusable file path input with Browse button + recent files.
 *
 * Replaces the 22+ bare text inputs across dialogs that require the user
 * to type file paths by hand. This was the #1 friction point identified
 * in the UX Researcher audit (docs/UX_RESEARCHER_AUDIT.md).
 *
 * Features:
 *   - Text input (still typeable for power users)
 *   - "Browse" button that opens the native OS file picker
 *   - File-type filtering (e.g., only show .las/.laz files)
 *   - Recent files dropdown (last 10 paths for this input type)
 *   - "Save" path button to persist the current path
 *   - Red border if the path doesn't look valid
 *
 * Usage:
 * ```tsx
 * <FileInput
 *   value={filePath}
 *   onChange={setFilePath}
 *   extensions={["las", "laz"]}
 *   filterName="LAS Point Cloud"
 *   storageKey="stockpile-audit-current"
 *   placeholder="/path/to/stockpile.las"
 * />
 * ```
 */

import { useState, useRef, useEffect } from "react";
import { FolderOpen, ChevronDown, X } from "lucide-react";
import { pickFile } from "@/lib/file-picker";
import { colors } from "@/lib/tokens";
import { isNative } from "@/lib/tauri-ipc";
import { Tooltip } from "@/components/tooltip";

interface FileInputProps {
  value: string;
  onChange: (path: string) => void;
  /** File extensions to filter (without dot, e.g., ["las", "laz"]). */
  extensions?: string[];
  /** Display name for the file type filter. */
  filterName?: string;
  /** Dialog title. */
  title?: string;
  /** Placeholder text. */
  placeholder?: string;
  /** localStorage key for recent files. If omitted, no recents are stored. */
  storageKey?: string;
  /** If true, this is a save path (uses save dialog). Default: false (open dialog). */
  save?: boolean;
  /** Additional className for the input. */
  className?: string;
}

interface RecentEntry {
  path: string;
  timestamp: number;
}

const MAX_RECENTS = 10;

function loadRecents(key: string): RecentEntry[] {
  try {
    const raw = localStorage.getItem(`metardu-recent-${key}`);
    return raw ? JSON.parse(raw) : [];
  } catch {
    return [];
  }
}

function saveRecent(key: string, path: string) {
  if (!path.trim()) return;
  try {
    const recents = loadRecents(key);
    const filtered = recents.filter((r) => r.path !== path);
    filtered.unshift({ path, timestamp: Date.now() });
    localStorage.setItem(`metardu-recent-${key}`, JSON.stringify(filtered.slice(0, MAX_RECENTS)));
  } catch {
    // Silently fail — localStorage full or unavailable
  }
}

export function FileInput({
  value,
  onChange,
  extensions,
  filterName,
  title,
  placeholder = "/path/to/file",
  storageKey,
  save = false,
  className = "",
}: FileInputProps) {
  const [browsing, setBrowsing] = useState(false);
  const [showRecents, setShowRecents] = useState(false);
  const [recents, setRecents] = useState<RecentEntry[]>([]);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (storageKey) {
      setRecents(loadRecents(storageKey));
    }
  }, [storageKey]);

  async function handleBrowse() {
    setBrowsing(true);
    try {
      const result = save
        ? await import("@tauri-apps/plugin-dialog").then((m) =>
            m.save({
              filters: extensions ? [{ name: filterName ?? "File", extensions }] : undefined,
              title: title ?? "Save file",
            }),
          )
        : await pickFile({
            extensions,
            filterName,
            title: title ?? "Select file",
          });
      if (result) {
        onChange(result);
        if (storageKey) {
          saveRecent(storageKey, result);
          setRecents(loadRecents(storageKey));
        }
      }
    } catch (err) {
      console.error("File picker error:", err);
    } finally {
      setBrowsing(false);
    }
  }

  function handleRecentSelect(path: string) {
    onChange(path);
    setShowRecents(false);
  }

  // Show red border if the path looks invalid (non-empty but no extension)
  const looksInvalid = value.trim().length > 0 && !value.includes(".") && !value.startsWith("flat:") && !value.startsWith("dxf:");

  return (
    <div className={`relative flex gap-1.5 ${className}`}>
      <div className="relative flex-1">
        <input
          ref={inputRef}
          type="text"
          value={value}
          onChange={(e) => onChange(e.target.value)}
          placeholder={placeholder}
          className="w-full rounded-md border bg-navy-base px-3 py-1.5 font-mono text-xs text-white focus:outline-none"
          style={{
            borderColor: looksInvalid ? colors.fail : colors.navyBorder,
          }}
          onFocus={() => storageKey && setShowRecents(true)}
          onBlur={() => setTimeout(() => setShowRecents(false), 200)}
        />
        {/* Recent files dropdown */}
        {showRecents && recents.length > 0 && (
          <div
            className="absolute left-0 right-0 top-full z-50 mt-1 rounded-md border border-navy-border bg-navy-panel shadow-lg max-h-48 overflow-y-auto"
          >
            <div className="flex items-center justify-between px-2 py-1 border-b border-navy-border">
              <span className="text-[9px] font-semibold uppercase tracking-wider text-steel-gray">
                Recent Files
              </span>
              <button
                onClick={() => setShowRecents(false)}
                className="text-steel-gray hover:text-white"
                aria-label="Close recent files"
              >
                <X className="h-3 w-3" />
              </button>
            </div>
            {recents.map((r, i) => (
              <button
                key={i}
                onMouseDown={(e) => {
                  e.preventDefault();
                  handleRecentSelect(r.path);
                }}
                className="flex w-full items-center gap-2 px-2 py-1.5 text-left text-[10px] font-mono text-steel-light hover:bg-navy-elevated"
              >
                <FolderOpen className="h-3 w-3 flex-shrink-0" style={{ color: colors.accent }} />
                <span className="truncate flex-1">{r.path}</span>
                <span className="text-steel-gray flex-shrink-0">
                  {new Date(r.timestamp).toLocaleDateString()}
                </span>
              </button>
            ))}
          </div>
        )}
      </div>

      <Tooltip text={isNative() ? "Browse files" : "Browser mode — file picker unavailable"} position="top">
        <button
          onClick={handleBrowse}
          disabled={browsing || !isNative()}
          className="flex items-center gap-1 rounded-md px-2.5 py-1.5 text-[10px] font-medium disabled:opacity-40"
          style={{ background: colors.steelGray, color: colors.navyBase }}
          aria-label="Browse for file"
        >
          <FolderOpen className="h-3 w-3" />
          <span className="hidden sm:inline">Browse</span>
        </button>
      </Tooltip>

      {/* Recents toggle button (only if storageKey provided) */}
      {storageKey && recents.length > 0 && (
        <Tooltip text="Show recent files" position="top">
          <button
            onClick={() => setShowRecents((v) => !v)}
            className="flex items-center rounded-md border border-navy-border px-2 py-1.5 text-steel-gray hover:bg-navy-elevated hover:text-white"
            aria-label="Toggle recent files"
          >
            <ChevronDown className="h-3 w-3" />
          </button>
        </Tooltip>
      )}
    </div>
  );
}
