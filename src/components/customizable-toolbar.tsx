/**
 * Customizable Toolbar — Sprint 13.
 *
 * Renders the user's pinned actions as a horizontal toolbar below the
 * title bar. Each button is a one-click shortcut to open a dialog.
 *
 * The toolbar shows a "customize" (gear) button at the right end that
 * opens the CustomizeToolbarDialog.
 */

import * as Icons from "lucide-react";
import { colors } from "@/lib/tokens";
import { useToolbarStore, AVAILABLE_ACTIONS } from "@/stores/toolbar-store";
import { Tooltip } from "@/components/tooltip";

interface Props {
  onOpenDialog: (dialogKey: string) => void;
  onCustomize: () => void;
}

export function CustomizableToolbar({ onOpenDialog, onCustomize }: Props) {
  const { pinned } = useToolbarStore();

  const pinnedActions = pinned
    .map((id) => AVAILABLE_ACTIONS.find((a) => a.id === id))
    .filter((a): a is NonNullable<typeof a> => a != null);

  return (
    <div className="toolbar-enterprise flex items-center gap-1 border-b border-navy-border px-2 py-1 overflow-x-auto">
      {pinnedActions.map((action) => {
        const Icon = (Icons as unknown as Record<string, Icons.LucideIcon>)[action.icon] || Icons.Circle;
        return (
          <Tooltip key={action.id} text={action.label} position="bottom" delay={400}>
            <button
              onClick={() => onOpenDialog(action.dialogKey)}
              className="flex items-center gap-1.5 rounded-md px-2.5 py-1.5 text-[11px] font-medium text-steel-light hover:bg-navy-elevated hover:text-white transition-colors flex-shrink-0"
              aria-label={action.label}
            >
              <Icon className="h-3.5 w-3.5" style={{ color: colors.accent }} />
              <span className="hidden sm:inline">{action.label}</span>
            </button>
          </Tooltip>
        );
      })}

      <div className="flex-1" />

      <Tooltip text="Customize toolbar" position="bottom">
        <button
          onClick={onCustomize}
          className="flex items-center gap-1 rounded-md px-2 py-1.5 text-steel-gray hover:bg-navy-elevated hover:text-white transition-colors"
          aria-label="Customize toolbar"
        >
          <Icons.Settings2 className="h-3.5 w-3.5" />
        </button>
      </Tooltip>
    </div>
  );
}
