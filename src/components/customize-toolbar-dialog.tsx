/**
 * Customize Toolbar Dialog — Sprint 13.
 *
 * Lets the user pin/unpin actions to the top toolbar. Shows all available
 * actions with a checkbox; pinned actions show at the top in toolbar order.
 *
 * The toolbar itself is rendered by the `CustomizableToolbar` component
 * in workspace-shell.tsx.
 */

import { Plus, Check, RotateCcw } from "lucide-react";
import { colors } from "@/lib/tokens";
import { useToolbarStore, AVAILABLE_ACTIONS } from "@/stores/toolbar-store";
import { useEscapeKey } from "@/lib/use-escape-key";
import { DialogShell, DialogButton } from "@/components/dialog-shell";

interface Props {
  open: boolean;
  onClose: () => void;
}

export function CustomizeToolbarDialog({ open, onClose }: Props) {
  const { pinned, pin, unpin, reset } = useToolbarStore();

  useEscapeKey(onClose, open);

  return (
    <DialogShell
      open={open}
      onClose={onClose}
      title="Customize Toolbar"
      icon={<Plus className="h-4 w-4" />}
      iconColor={colors.accent}
      maxWidth="max-w-lg"
      subtitle={`${pinned.length} actions pinned`}
      footerHint="Pinned actions show in the top toolbar for one-click access"
      actions={
        <>
          <DialogButton variant="secondary" onClick={reset}>
            <RotateCcw className="h-3 w-3" /> Reset to defaults
          </DialogButton>
          <DialogButton variant="secondary" onClick={onClose}>Done</DialogButton>
        </>
      }
    >
      <div className="space-y-1">
        {AVAILABLE_ACTIONS.map((action) => {
          const isPinned = pinned.includes(action.id);
          return (
            <button
              key={action.id}
              onClick={() => (isPinned ? unpin(action.id) : pin(action.id))}
              className="flex w-full items-center gap-3 rounded-md border border-navy-border bg-navy-base p-2.5 text-left transition-colors hover:border-accent"
              style={{
                borderColor: isPinned ? `${colors.accent}60` : colors.navyBorder,
                background: isPinned ? `${colors.accent}08` : colors.navyBase,
              }}
            >
              <div
                className="flex h-5 w-5 items-center justify-center rounded border"
                style={{
                  borderColor: isPinned ? colors.accent : colors.border,
                  background: isPinned ? colors.accent : "transparent",
                }}
              >
                {isPinned && <Check className="h-3 w-3" style={{ color: colors.navyBase }} />}
              </div>
              <div className="flex-1">
                <div className="text-sm font-medium text-white">{action.label}</div>
                <div className="text-[10px] text-steel-gray font-mono">{action.icon}</div>
              </div>
              {isPinned && (
                <span className="text-[10px] font-medium" style={{ color: colors.accent }}>
                  #{pinned.indexOf(action.id) + 1}
                </span>
              )}
            </button>
          );
        })}
      </div>
    </DialogShell>
  );
}
