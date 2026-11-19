import { useEffect, useState } from "react";
import type Map from "ol/Map";
import { Clock, Crosshair, MapPin, Redo2, Undo2 } from "lucide-react";
import { APP_VERSION, domainAccent, type DomainMode } from "@/lib/tokens";
import { useUndoStore } from "@/stores/undo-store";

interface WorkspaceStatusBarProps {
  domain: DomainMode;
  epsg: string;
  map: Map | null;
  onOpenShortcuts: () => void;
}

export function WorkspaceStatusBar({ domain, epsg, map, onOpenShortcuts }: WorkspaceStatusBarProps) {
  const accent = domainAccent[domain].primary;
  const [utcTime, setUtcTime] = useState(() => new Date().toISOString().slice(11, 19));
  const [coords, setCoords] = useState("—");
  const [scale, setScale] = useState("—");
  const undoDepth = useUndoStore((state) => state.undoStack.length);
  const redoDepth = useUndoStore((state) => state.redoStack.length);
  const peekUndoDesc = useUndoStore((state) => state.undoStack.at(-1)?.description ?? null);
  const undo = useUndoStore((state) => state.undo);
  const redo = useUndoStore((state) => state.redo);

  useEffect(() => {
    const timer = window.setInterval(() => setUtcTime(new Date().toISOString().slice(11, 19)), 1000);
    return () => window.clearInterval(timer);
  }, []);

  useEffect(() => {
    if (!map) return;
    const onMouseMove = (event: { coordinate: number[] }) => {
      const [x, y] = event.coordinate;
      setCoords(`${x.toFixed(6)}, ${y.toFixed(6)}`);
      const resolution = map.getView().getResolution();
      if (resolution) setScale(`1:${Math.round(resolution * 96 / 0.0254).toLocaleString()}`);
    };
    map.on("pointermove", onMouseMove);
    return () => map.un("pointermove", onMouseMove);
  }, [map]);

  return (
    <footer className="status-bar status-bar-enterprise" aria-label="Survey telemetry">
      <div className="status-bar-item font-medium text-emerald-400"><span className="h-1.5 w-1.5 rounded-full bg-emerald-400 animate-pulse" /><span>Ready</span></div>
      <Divider />
      <div className="status-bar-item" title="Cursor coordinates"><Crosshair className="h-3 w-3 text-[var(--color-text-secondary)]" /><span className="font-mono text-[10px] text-[var(--color-white)]">{coords}</span></div>
      <Divider />
      <div className="status-bar-item"><span>Scale</span><span className="font-mono text-[10px] text-[var(--color-white)]">{scale}</span></div>
      <Divider />
      <div className="status-bar-item"><MapPin className="h-3 w-3 text-[var(--color-text-secondary)]" /><span className="font-mono text-[10px] text-[var(--color-white)]">{epsg}</span></div>
      <Divider />
      <div className="status-bar-item"><span className="rounded-sm border border-[var(--color-border)] bg-[var(--color-elevated)] px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-wider" style={{ color: accent }}>{domainAccent[domain].label}</span></div>
      <Divider />
      <div className="status-bar-item" title={peekUndoDesc ? `Undo: ${peekUndoDesc}` : "No actions to undo"}>
        <button type="button" onClick={undo} disabled={undoDepth === 0} title={peekUndoDesc ? `Undo: ${peekUndoDesc} (Ctrl+Z)` : "Nothing to undo"} aria-label="Undo" className="rounded-sm p-1 text-[var(--color-text-secondary)] hover:bg-[var(--color-elevated)] hover:text-[var(--color-white)] disabled:opacity-30"><Undo2 className="h-3 w-3" /></button>
        <span className="font-mono text-[10px]">{undoDepth}</span>
        <button type="button" onClick={redo} disabled={redoDepth === 0} title="Redo (Ctrl+Y)" aria-label="Redo" className="rounded-sm p-1 text-[var(--color-text-secondary)] hover:bg-[var(--color-elevated)] hover:text-[var(--color-white)] disabled:opacity-30"><Redo2 className="h-3 w-3" /></button>
        <span className="font-mono text-[10px]">{redoDepth}</span>
      </div>
      <div className="flex-1" />
      <div className="status-bar-item"><kbd>Ctrl+K</kbd><span>Commands</span></div>
      <button type="button" className="status-bar-item cursor-pointer hover:text-[var(--color-white)]" onClick={onOpenShortcuts} title="Show keyboard shortcuts (?)"><kbd>?</kbd><span>Shortcuts</span></button>
      <Divider />
      <div className="status-bar-item"><Clock className="h-3 w-3 text-[var(--color-text-secondary)]" /><span className="font-mono text-[10px] tabular-nums">{utcTime}Z</span></div>
      <div className="status-bar-item font-mono text-[10px]">v{APP_VERSION}</div>
    </footer>
  );
}

function Divider() {
  return <div aria-hidden="true" className="h-3 w-px bg-[var(--color-border)]" />;
}
