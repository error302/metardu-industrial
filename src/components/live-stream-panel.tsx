/**
 * Live Stream Panel — Phase 4.
 *
 * Subscribes to 'stream://pings' events from the Rust streaming listener
 * and renders pings on the map in real-time. Also shows rate stats
 * (pings/sec, total received, buffer status).
 */

import { useEffect, useState, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { Radio } from "lucide-react";
import { colors } from "@/lib/tokens";

interface StreamPing {
  x: number;
  y: number;
  depth: number;
  uncertainty: number;
  timestamp: number;
}

interface Props {
  isStreaming: boolean;
  onPings: (pings: StreamPing[]) => void;
}

export function LiveStreamPanel({ isStreaming, onPings }: Props) {
  const [pingsReceived, setPingsReceived] = useState(0);
  const [rate, setRate] = useState(0);
  const [buffered, setBuffered] = useState(0);
  const lastUpdateRef = useRef<number>(0);
  const lastCountRef = useRef<number>(0);

  useEffect(() => {
    if (!isStreaming) return;

    const unlisten = listen<StreamPing[]>("stream://pings", (event) => {
      const pings = event.payload;
      if (pings.length > 0) {
        onPings(pings);
        setPingsReceived((prev) => prev + pings.length);
        setBuffered(pings.length);

        // Calculate rate
        const now = Date.now();
        const elapsed = (now - lastUpdateRef.current) / 1000;
        if (elapsed >= 1.0) {
          const countDelta = pingsReceived - lastCountRef.current;
          setRate(countDelta / elapsed);
          lastUpdateRef.current = now;
          lastCountRef.current = pingsReceived;
        }
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [isStreaming, onPings, pingsReceived]);

  if (!isStreaming) return null;

  return (
    <div
      className="pointer-events-auto absolute right-3 bottom-12 z-20 rounded-md border bg-navy-base/90 px-3 py-2 backdrop-blur"
      style={{ borderColor: `${colors.marineTurquoise}60` }}
    >
      <div className="flex items-center gap-2">
        <Radio className="h-3.5 w-3.5 animate-pulse" style={{ color: colors.marineTurquoise }} />
        <span className="text-[10px] font-semibold uppercase tracking-wider" style={{ color: colors.marineTurquoise }}>
          Live Stream
        </span>
        <span className="font-mono text-[10px] text-steel-light">
          {rate.toFixed(1)} p/s
        </span>
      </div>
      <div className="mt-1 flex items-center gap-3 text-[9px] text-steel-gray">
        <span>Total: <span className="font-mono text-steel-light">{pingsReceived.toLocaleString()}</span></span>
        <span>Buffer: <span className="font-mono text-steel-light">{buffered}</span></span>
      </div>
    </div>
  );
}
