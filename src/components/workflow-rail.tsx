import {
  ClipboardCheck,
  Database,
  FileCheck2,
  Layers3,
  Play,
  ShieldCheck,
} from "lucide-react";
import { domainAccent, type DomainMode } from "@/lib/tokens";

export type WorkflowStage = "ingest" | "validate" | "process" | "review" | "deliver";

export interface WorkflowActions {
  ingest: () => void;
  validate: () => void;
  process: () => void;
  review: () => void;
  deliver: () => void;
}

interface WorkflowRailProps {
  domain: DomainMode;
  stage: WorkflowStage;
  onStageChange: (stage: WorkflowStage) => void;
  actions: WorkflowActions;
}

const stages: Array<{
  id: WorkflowStage;
  label: string;
  mining: string;
  marine: string;
  both: string;
  icon: typeof Database;
}> = [
  { id: "ingest", label: "Ingest", mining: "Stage survey data", marine: "Stage survey data", both: "Stage survey data", icon: Database },
  { id: "validate", label: "Validate", mining: "Classify ground", marine: "Check S-44 readiness", both: "Run domain checks", icon: ShieldCheck },
  { id: "process", label: "Process", mining: "Compute volumes", marine: "Build CUBE surface", both: "Create survey outputs", icon: Layers3 },
  { id: "review", label: "Review", mining: "Audit results", marine: "Review compliance", both: "Review results", icon: ClipboardCheck },
  { id: "deliver", label: "Deliver", mining: "Package deliverables", marine: "Package deliverables", both: "Package deliverables", icon: FileCheck2 },
];

function stageAction(stage: (typeof stages)[number], domain: DomainMode): string {
  return stage[domain];
}

export function WorkflowRail({ domain, stage, onStageChange, actions }: WorkflowRailProps) {
  const accent = domainAccent[domain].primary;
  const activeStage = stages.find((item) => item.id === stage) ?? stages[0];

  return (
    <nav
      aria-label="Survey workflow"
      className="flex min-h-10 items-center gap-1 overflow-x-auto border-b border-[var(--color-border)] bg-[var(--color-panel)] px-3 py-1.5"
    >
      <div className="flex min-w-max items-center gap-1">
        {stages.map((item, index) => {
          const Icon = item.icon;
          const isActive = item.id === stage;
          return (
            <div key={item.id} className="flex items-center gap-1">
              {index > 0 && <div aria-hidden="true" className="h-px w-3 bg-[var(--color-border)]" />}
              <button
                type="button"
                aria-current={isActive ? "step" : undefined}
                aria-label={item.label}
                onClick={() => onStageChange(item.id)}
                className={`flex items-center gap-1.5 rounded-sm px-2 py-1 text-[11px] font-medium transition-colors focus-visible:outline-offset-2 ${
                  isActive
                    ? "bg-[var(--color-elevated)] text-[var(--color-white)]"
                    : "text-[var(--color-text-secondary)] hover:bg-[var(--color-elevated)] hover:text-[var(--color-white)]"
                }`}
                style={isActive ? { boxShadow: `inset 0 -2px 0 ${accent}` } : undefined}
              >
                <Icon className="h-3.5 w-3.5" style={isActive ? { color: accent } : undefined} />
                <span>{item.label}</span>
              </button>
            </div>
          );
        })}
      </div>

      <div className="ml-auto flex min-w-max items-center gap-2 pl-3">
        <span className="hidden text-[11px] text-[var(--color-text-secondary)] xl:inline">
          {stageAction(activeStage, domain)}
        </span>
        <button
          type="button"
          onClick={actions[stage]}
          className="flex items-center gap-1.5 rounded-sm bg-[var(--color-accent)] px-2.5 py-1 text-[11px] font-semibold text-[var(--color-base)] transition-colors hover:bg-[var(--color-accent-light)]"
        >
          <Play className="h-3 w-3 fill-current" />
          <span>Open {activeStage.label}</span>
        </button>
      </div>
    </nav>
  );
}
