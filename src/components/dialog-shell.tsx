/**
 * DialogShell — reusable dialog container (Sprint 12 UI consistency).
 *
 * Before this component, every dialog in MetaRDU repeated ~40 lines of
 * overlay + header + body + footer boilerplate. The 45 dialogs had
 * 6 different button-padding variants, 5 different max-height values,
 * and inconsistent close-button placement.
 *
 * DialogShell enforces:
 *   - Consistent overlay (bg-black/60 backdrop-blur, click-outside-to-close)
 *   - Consistent header (icon + title + close button, border-b)
 *   - Scrollable body (flex-1 overflow-y-auto p-5)
 *   - Consistent footer (border-t, left hint + right action buttons)
 *   - Standard max-height (88vh)
 *   - ESC-to-close via useEscapeKey
 *   - Body scroll lock (via the workspace shell's has-open-dialog class)
 *
 * Usage:
 * ```tsx
 * <DialogShell
 *   open={open}
 *   onClose={onClose}
 *   title="Volume Calculator"
 *   icon={<Calculator className="h-4 w-4" />}
 *   iconColor={colors.mining}
 *   maxWidth="max-w-2xl"
 *   footerHint="Grid method · cell size 1.0 m"
 *   actions={
 *     <>
 *       <DialogButton variant="secondary" onClick={onClose}>Close</DialogButton>
 *       <DialogButton variant="primary" onClick={handleCompute}>Compute</DialogButton>
 *     </>
 *   }
 * >
 *   {body content}
 * </DialogShell>
 * ```
 */

import type { ReactNode } from "react";
import { useEffect, useRef } from "react";
import { X } from "lucide-react";
import { colors } from "@/lib/tokens";
import { useEscapeKey } from "@/lib/use-escape-key";

interface DialogShellProps {
  open: boolean;
  onClose: () => void;
  title: string;
  icon?: ReactNode;
  iconColor?: string;
  /** Tailwind max-width class. Default: "max-w-2xl". */
  maxWidth?: string;
  /** Optional sub-title shown under the main title in muted text. */
  subtitle?: string;
  /** Children rendered in the scrollable body. */
  children: ReactNode;
  /** Left-aligned hint text in the footer (e.g., method + parameters). */
  footerHint?: string;
  /** Right-aligned action buttons. Use <DialogButton>. */
  actions?: ReactNode;
  /** If true, clicking the backdrop does NOT close the dialog. Default: false. */
  disableBackdropClose?: boolean;
}

export function DialogShell({
  open,
  onClose,
  title,
  icon,
  iconColor = colors.steelLight,
  maxWidth = "max-w-2xl",
  subtitle,
  children,
  footerHint,
  actions,
  disableBackdropClose = false,
}: DialogShellProps) {
  useEscapeKey(onClose, open);
  const dialogRef = useRef<HTMLDivElement>(null);

  // Focus trap — when the dialog is open, Tab cycles within the dialog
  // (Sprint 19 WCAG fix). Also moves focus into the dialog on open and
  // returns focus to the trigger on close.
  useEffect(() => {
    if (!open) return;

    const dialog = dialogRef.current;
    if (!dialog) return;

    // Move focus into the dialog
    const previouslyFocused = document.activeElement as HTMLElement | null;
    const firstFocusable = dialog.querySelector<HTMLElement>(
      'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
    );
    firstFocusable?.focus();

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key !== "Tab") return;

      const focusable = dialog.querySelectorAll<HTMLElement>(
        'button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])'
      );
      if (focusable.length === 0) return;

      const first = focusable[0];
      const last = focusable[focusable.length - 1];

      if (e.shiftKey && document.activeElement === first) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && document.activeElement === last) {
        e.preventDefault();
        first.focus();
      }
    };

    dialog.addEventListener("keydown", handleKeyDown);

    return () => {
      dialog.removeEventListener("keydown", handleKeyDown);
      // Return focus to the trigger element
      previouslyFocused?.focus();
    };
  }, [open]);

  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
      onClick={() => {
        if (!disableBackdropClose) onClose();
      }}
    >
      <div
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby="dialog-title"
        onClick={(e) => e.stopPropagation()}
        className={`dialog-enter-enterprise flex max-h-[88vh] w-full ${maxWidth} flex-col rounded-lg border border-navy-border bg-navy-panel shadow-2xl`}
      >
        {/* Header */}
        <div className="flex items-center justify-between border-b border-navy-border px-5 py-3">
          <div className="flex items-center gap-2 min-w-0">
            {icon && <span className="flex-shrink-0" style={{ color: iconColor }}>{icon}</span>}
            <div className="min-w-0">
              <h2 id="dialog-title" className="text-sm font-semibold text-white truncate">{title}</h2>
              {subtitle && (
                <p className="text-[10px] text-steel-gray truncate">{subtitle}</p>
              )}
            </div>
          </div>
          <button
            onClick={onClose}
            aria-label="Close dialog"
            title="Close (Esc)"
            className="rounded p-1 text-steel-gray hover:bg-navy-elevated hover:text-white flex-shrink-0"
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        {/* Body */}
        <div className="flex-1 overflow-y-auto p-5">{children}</div>

        {/* Footer — only render if there's a hint or actions */}
        {(footerHint || actions) && (
          <div className="flex items-center justify-between gap-3 border-t border-navy-border px-5 py-3">
            <div className="text-[10px] text-steel-gray truncate min-w-0">
              {footerHint}
            </div>
            {actions && <div className="flex gap-2 flex-shrink-0">{actions}</div>}
          </div>
        )}
      </div>
    </div>
  );
}

// ──────────────────────────────────────────────────────────────────
// DialogButton — consistent button styling across all dialogs
// ──────────────────────────────────────────────────────────────────

type DialogButtonVariant = "primary" | "secondary" | "danger" | "success" | "marine";

interface DialogButtonProps {
  variant?: DialogButtonVariant;
  onClick: () => void;
  disabled?: boolean;
  title?: string;
  ariaLabel?: string;
  children: ReactNode;
}

/**
 * Standard dialog button. Eliminates the 6 padding variants that
 * existed before Sprint 12. All buttons are now `px-4 py-1.5 text-xs
 * font-medium rounded-md`.
 */
export function DialogButton({
  variant = "secondary",
  onClick,
  disabled,
  title,
  ariaLabel,
  children,
}: DialogButtonProps) {
  const styles: Record<DialogButtonVariant, { bg: string; color: string }> = {
    primary: { bg: colors.accent, color: colors.navyBase },
    secondary: { bg: colors.steelGray, color: colors.navyBase },
    danger: { bg: colors.fail, color: colors.white },
    success: { bg: colors.pass, color: colors.navyBase },
    marine: { bg: colors.marine, color: colors.navyBase },
  };
  const { bg, color } = styles[variant];
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      title={title}
      aria-label={ariaLabel ?? (typeof children === "string" ? children : undefined)}
      className="btn-enterprise flex items-center gap-1.5 rounded-md px-4 py-1.5 text-xs font-medium disabled:opacity-40 disabled:cursor-not-allowed transition-opacity"
      style={{ background: bg, color }}
    >
      {children}
    </button>
  );
}

// ──────────────────────────────────────────────────────────────────
// EmptyState — consistent "no data" display
// ──────────────────────────────────────────────────────────────────

interface EmptyStateProps {
  icon: ReactNode;
  title: string;
  description?: string;
  action?: ReactNode;
}

/**
 * Empty state — shown when a dialog or panel has no data to display.
 * Replaces the inconsistent "No data" / "no files" / bare text patterns.
 */
export function EmptyState({ icon, title, description, action }: EmptyStateProps) {
  return (
    <div className="flex flex-col items-center justify-center py-12 px-4 text-center">
      <div className="mb-3 text-steel-gray opacity-60">{icon}</div>
      <h3 className="text-sm font-semibold text-steel-light mb-1">{title}</h3>
      {description && <p className="text-[10px] text-steel-gray max-w-xs">{description}</p>}
      {action && <div className="mt-4">{action}</div>}
    </div>
  );
}

// ──────────────────────────────────────────────────────────────────
// LoadingSkeleton — shimmer placeholder for async content
// ──────────────────────────────────────────────────────────────────

/**
 * Loading skeleton — shimmer placeholder shown while async data loads.
 * Use instead of bare "Loading..." text.
 */
export function LoadingSkeleton({ lines = 4 }: { lines?: number }) {
  return (
    <div className="space-y-2 animate-pulse">
      {Array.from({ length: lines }).map((_, i) => (
        <div
          key={i}
          className="h-4 rounded bg-navy-elevated"
          style={{ width: `${85 - (i % 3) * 15}%` }}
        />
      ))}
    </div>
  );
}
