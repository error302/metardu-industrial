/**
 * MBES Survey Reader — Sprint 10 Marine Tool #4.
 *
 * Ingests a Kongsberg .all file and surfaces bathymetry, position,
 * attitude, and water-column statistics. Acts as the entry point
 * for the other marine dialogs (QC dashboard, backscatter mosaic).
 *
 * Workflow:
 *   1. Browse or paste .all path
 *   2. Click Load → see survey metadata + bounds + ping/position/attitude counts
 *   3. Tab through Bathymetry / Position / Attitude / Water Column previews
 *   4. Click "Open in QC Dashboard" / "Open in Backscatter Mosaic" to hand off
 */

import { useState, useMemo } from "react";
import { FileSearch, Loader2, Waves, MapPin, Compass, Droplets } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { colors } from "@/lib/tokens";
import { isNative } from "@/lib/tauri-ipc";
import { DialogShell, DialogButton } from "@/components/dialog-shell";

interface AllHeader {
  model: string;
  model_id: number;
  date: string;
  seconds_since_epoch: number;
  ping_count: number;
  position_count: number;
  attitude_count: number;
  svp_count: number;
  runtime_count: number;
  total_datagrams: number;
}

interface KongsbergSounding {
  timestamp: number;
  ping_number: number;
  beam_number: number;
  depth: number;
  across_track: number;
  along_track?: number;
  quality?: number;
}

interface KongsbergPosition {
  timestamp: number;
  latitude: number;
  longitude: number;
  height: number;
  quality: number;
}

interface KongsbergAttitude {
  timestamp: number;
  roll: number;
  pitch: number;
  heave: number;
  heading: number;
}

interface AllSurveyData {
  header: AllHeader;
  soundings: KongsbergSounding[];
  positions: KongsbergPosition[];
  attitudes: KongsbergAttitude[];
  bounds: [number, number, number, number] | null;
}

interface WaterColumnSummary {
  ping_count: number;
  total_samples: number;
  max_samples_per_beam: number;
  beams_per_ping: number;
}

interface Props {
  open: boolean;
  onClose: () => void;
  onOpenQc?: () => void;
  onOpenBackscatter?: () => void;
}

type Tab = "bathymetry" | "position" | "attitude" | "water_column";

export function MbesSurveyDialog({ open, onClose, onOpenQc, onOpenBackscatter }: Props) {
  const [filePath, setFilePath] = useState("");
  const [maxPings, setMaxPings] = useState(0);
  const [survey, setSurvey] = useState<AllSurveyData | null>(null);
  const [wcSummary, setWcSummary] = useState<WaterColumnSummary | null>(null);
  const [tab, setTab] = useState<Tab>("bathymetry");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);


  async function handleLoad() {
    setLoading(true);
    setError(null);
    setSurvey(null);
    setWcSummary(null);
    try {
      if (!isNative()) {
        setError("Browser mode — .all parsing requires the native Tauri shell");
        return;
      }
      const result = await invoke<AllSurveyData>("read_all_survey_cmd", {
        path: filePath,
        maxPings,
      });
      setSurvey(result);
      // Try water column extraction too (best-effort — may not exist in file)
      try {
        const wc = await invoke<WaterColumnSummary>("extract_water_column_summary_cmd", {
          path: filePath,
          maxPings,
        });
        setWcSummary(wc);
      } catch {
        // Water column datagrams absent — that's OK
      }
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }

  // Render bathymetry preview (depth vs beam number for first 200 soundings)
  const bathyPreview = useMemo(() => {
    if (!survey || survey.soundings.length === 0) return null;
    const sample = survey.soundings.slice(0, Math.min(500, survey.soundings.length));
    const depths = sample.map((s) => s.depth);
    const minD = Math.min(...depths);
    const maxD = Math.max(...depths);
    const range = Math.max(0.001, maxD - minD);
    const W = 600, H = 200, pad = 30;
    const points = sample.map((s, i) => {
      const x = pad + (i / sample.length) * (W - 2 * pad);
      const y = pad + ((s.depth - minD) / range) * (H - 2 * pad);
      return { x, y, depth: s.depth };
    });
    return { points, minD, maxD, W, H, pad };
  }, [survey]);

return (
    <DialogShell
      open={open}
      onClose={onClose}
      title="MBES Survey Reader"
      icon={<FileSearch className="h-4 w-4" />}
      iconColor={colors.marineTurquoise}
      maxWidth="max-w-5xl"
      subtitle="Kongsberg .all ingest"
      footerHint="Bathymetry + position + attitude"
      actions={
        <DialogButton variant="secondary" onClick={onClose}>Close</DialogButton>
      }
    >
          {/* File input */}
          <div className="grid grid-cols-[1fr_140px_auto] items-end gap-3">
            <div>
              <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">
                Kongsberg .all file path
              </label>
              <input
                type="text"
                value={filePath}
                onChange={(e) => setFilePath(e.target.value)}
                placeholder="/path/to/survey.all"
                className="w-full rounded-md border border-navy-border bg-navy-base px-2 py-1.5 font-mono text-xs text-white focus:border-marine focus:outline-none"
                onKeyDown={(e) => e.key === "Enter" && handleLoad()}
              />
            </div>
            <div>
              <label className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-steel-gray">Max Pings (0=all)</label>
              <input
                type="number"
                value={maxPings}
                min={0}
                step={100}
                onChange={(e) => setMaxPings(Math.max(0, parseInt(e.target.value) || 0))}
                className="w-full rounded-md border border-navy-border bg-navy-base px-2 py-1.5 font-mono text-sm text-white focus:border-marine focus:outline-none"
              />
            </div>
            <button
              onClick={handleLoad}
              disabled={loading || !filePath.trim()}
              className="flex items-center gap-1.5 rounded-md px-4 py-1.5 text-xs font-medium disabled:opacity-40"
              style={{ background: colors.marine, color: colors.navyBase }}
            >
              {loading ? <Loader2 className="h-3 w-3 animate-spin" /> : <FileSearch className="h-3 w-3" />}
              {loading ? "Loading…" : "Load Survey"}
            </button>
          </div>

          {error && (
            <div className="rounded-md border p-3 text-xs" style={{ borderColor: `${colors.fail}40`, background: `${colors.fail}10`, color: colors.fail }}>
              {error}
            </div>
          )}

          {survey && (
            <>
              {/* Header stats */}
              <div className="grid grid-cols-4 gap-2">
                <Kpi icon={<Waves className="h-3 w-3" />} label="Sonar Model" value={survey.header.model} color={colors.marine} />
                <Kpi icon={<FileSearch className="h-3 w-3" />} label="Total Datagrams" value={survey.header.total_datagrams.toLocaleString()} color={colors.steelLight} />
                <Kpi icon={<FileSearch className="h-3 w-3" />} label="Pings" value={survey.header.ping_count.toLocaleString()} color={colors.steelLight} />
                <Kpi icon={<FileSearch className="h-3 w-3" />} label="SVPs" value={survey.header.svp_count.toLocaleString()} color={colors.steelLight} />
              </div>

              {/* Bounds */}
              {survey.bounds && (
                <div className="rounded-md border border-navy-border bg-navy-base p-3 font-mono text-xs text-steel-light">
                  <span className="text-steel-gray">Bounds (lon/lat):</span>{" "}
                  {survey.bounds[0].toFixed(5)}, {survey.bounds[1].toFixed(5)} → {survey.bounds[2].toFixed(5)}, {survey.bounds[3].toFixed(5)}
                </div>
              )}

              {/* Tabs */}
              <div className="flex gap-1 border-b border-navy-border">
                {([
                  { id: "bathymetry" as Tab, label: "Bathymetry", icon: Waves, count: survey.soundings.length },
                  { id: "position" as Tab, label: "Position", icon: MapPin, count: survey.positions.length },
                  { id: "attitude" as Tab, label: "Attitude", icon: Compass, count: survey.attitudes.length },
                  { id: "water_column" as Tab, label: "Water Column", icon: Droplets, count: wcSummary?.total_samples ?? 0 },
                ]).map((t) => (
                  <button
                    key={t.id}
                    onClick={() => setTab(t.id)}
                    className={`flex items-center gap-1.5 rounded-t-md px-3 py-2 text-xs font-medium ${tab === t.id ? "border-b-2" : "text-steel-gray"}`}
                    style={{
                      borderColor: tab === t.id ? colors.marine : "transparent",
                      color: tab === t.id ? colors.marine : colors.steelGray,
                    }}
                  >
                    <t.icon className="h-3 w-3" /> {t.label}
                    <span className="ml-1 rounded px-1.5 py-0.5 text-[9px]" style={{ background: colors.navyElevated, color: colors.steelLight }}>
                      {t.count.toLocaleString()}
                    </span>
                  </button>
                ))}
              </div>

              {/* Tab content */}
              <div className="min-h-[280px]">
                {tab === "bathymetry" && (
                  <div>
                    {bathyPreview ? (
                      <svg viewBox={`0 0 ${bathyPreview.W} ${bathyPreview.H}`} className="w-full" style={{ maxHeight: "260px" }}>
                        <line x1={bathyPreview.pad} y1={bathyPreview.H - bathyPreview.pad} x2={bathyPreview.W - bathyPreview.pad} y2={bathyPreview.H - bathyPreview.pad} stroke={colors.steelGray} strokeWidth="0.5" />
                        <line x1={bathyPreview.pad} y1={bathyPreview.pad} x2={bathyPreview.pad} y2={bathyPreview.H - bathyPreview.pad} stroke={colors.steelGray} strokeWidth="0.5" />
                        {bathyPreview.points.map((p, i) => (
                          <circle key={i} cx={p.x} cy={p.y} r="1.2" fill={colors.marine} />
                        ))}
                        <text x={bathyPreview.W / 2} y={bathyPreview.H - 5} textAnchor="middle" fill={colors.steelGray} fontSize="9" fontFamily="JetBrains Mono">
                          Sample (first {bathyPreview.points.length} soundings)
                        </text>
                        <text x={10} y={bathyPreview.H / 2} textAnchor="middle" fill={colors.steelGray} fontSize="9" fontFamily="JetBrains Mono"
                          transform={`rotate(-90, 10, ${bathyPreview.H / 2})`}>
                          Depth (m) — {bathyPreview.minD.toFixed(1)} to {bathyPreview.maxD.toFixed(1)}
                        </text>
                      </svg>
                    ) : (
                      <div className="text-[10px] text-steel-gray">No bathymetry soundings.</div>
                    )}
                  </div>
                )}

                {tab === "position" && (
                  <div>
                    {survey.positions.length > 0 ? (
                      <div className="grid grid-cols-2 gap-2">
                        {survey.positions.slice(0, 12).map((p, i) => (
                          <div key={i} className="rounded border border-navy-border bg-navy-base p-2 font-mono text-[10px] text-steel-light">
                            <div>lat {p.latitude.toFixed(6)}  lon {p.longitude.toFixed(6)}</div>
                            <div className="text-steel-gray">h {p.height.toFixed(2)} m · q{p.quality} · t {p.timestamp.toFixed(1)}</div>
                          </div>
                        ))}
                        {survey.positions.length > 12 && (
                          <div className="col-span-2 text-center text-[10px] text-steel-gray">
                            +{(survey.positions.length - 12).toLocaleString()} more positions
                          </div>
                        )}
                      </div>
                    ) : (
                      <div className="text-[10px] text-steel-gray">No position records.</div>
                    )}
                  </div>
                )}

                {tab === "attitude" && (
                  <div>
                    {survey.attitudes.length > 0 ? (
                      <svg viewBox="0 0 600 200" className="w-full" style={{ maxHeight: "260px" }}>
                        <line x1="30" y1="170" x2="570" y2="170" stroke={colors.steelGray} strokeWidth="0.5" />
                        <line x1="30" y1="20" x2="30" y2="170" stroke={colors.steelGray} strokeWidth="0.5" />
                        {survey.attitudes.slice(0, 300).map((a, i) => {
                          const x = 30 + (i / Math.min(300, survey.attitudes.length)) * 540;
                          const rollY = 100 - a.roll * 5;
                          const pitchY = 100 - a.pitch * 5;
                          const heaveY = 100 - a.heave * 30;
                          return (
                            <g key={i}>
                              <circle cx={x} cy={rollY} r="1" fill={colors.marine} />
                              <circle cx={x} cy={pitchY} r="1" fill={colors.mining} />
                              <circle cx={x} cy={heaveY} r="1" fill={colors.fail} />
                            </g>
                          );
                        })}
                        <text x="300" y="195" textAnchor="middle" fill={colors.steelGray} fontSize="9" fontFamily="JetBrains Mono">Sample index</text>
                      </svg>
                    ) : (
                      <div className="text-[10px] text-steel-gray">No attitude records.</div>
                    )}
                    <div className="mt-1 flex gap-3 text-[9px] text-steel-gray">
                      <span><span className="inline-block h-2 w-3 align-middle" style={{ background: colors.marine }} /> Roll</span>
                      <span><span className="inline-block h-2 w-3 align-middle" style={{ background: colors.mining }} /> Pitch</span>
                      <span><span className="inline-block h-2 w-3 align-middle" style={{ background: colors.fail }} /> Heave</span>
                    </div>
                  </div>
                )}

                {tab === "water_column" && (
                  <div>
                    {wcSummary && wcSummary.total_samples > 0 ? (
                      <div className="grid grid-cols-2 gap-2">
                        <Kpi icon={<Droplets className="h-3 w-3" />} label="WC Pings" value={wcSummary.ping_count.toLocaleString()} color={colors.marine} />
                        <Kpi icon={<Droplets className="h-3 w-3" />} label="Total Samples" value={wcSummary.total_samples.toLocaleString()} color={colors.marine} />
                        <Kpi icon={<Droplets className="h-3 w-3" />} label="Max Samples/Beam" value={wcSummary.max_samples_per_beam.toString()} color={colors.steelLight} />
                        <Kpi icon={<Droplets className="h-3 w-3" />} label="Beams/Ping" value={wcSummary.beams_per_ping.toString()} color={colors.steelLight} />
                      </div>
                    ) : (
                      <div className="rounded-md border border-navy-border bg-navy-base p-3 text-[10px] text-steel-gray">
                        No water-column datagrams (type 0x4D) found in this file. Water column data is only present
                        when the sonar was configured to record full acoustic returns — typical for object-detection
                        surveys but not for standard bathymetric surveys.
                      </div>
                    )}
                  </div>
                )}
              </div>

              {/* Hand-off buttons */}
              <div className="flex gap-2 border-t border-navy-border pt-3">
                <button
                  onClick={() => { onClose(); onOpenQc?.(); }}
                  className="rounded-md px-3 py-1.5 text-[10px] font-medium"
                  style={{ background: colors.marine, color: colors.navyBase }}
                >
                  → Open in QC Dashboard
                </button>
                <button
                  onClick={() => { onClose(); onOpenBackscatter?.(); }}
                  className="rounded-md px-3 py-1.5 text-[10px] font-medium"
                  style={{ background: colors.steelLight, color: colors.navyBase }}
                >
                  → Open in Backscatter Mosaic
                </button>
              </div>
            </>
          )}
    </DialogShell>
  );
}

function Kpi({ icon, label, value, color }: { icon: React.ReactNode; label: string; value: string; color: string }) {
  return (
    <div className="rounded-md border p-2" style={{ borderColor: `${color}40`, background: `${color}10` }}>
      <div className="flex items-center gap-1 text-[9px] uppercase tracking-wider" style={{ color }}>
        {icon} {label}
      </div>
      <div className="mt-0.5 font-mono text-sm font-bold text-white">{value}</div>
    </div>
  );
}
