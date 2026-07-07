# Brand Guardian — Visual Consistency Audit

**Agent**: Brand Guardian (activated from `skills/agency-agents/design/design-brand-guardian.md`)
**Date**: 2026-07-07
**Scope**: All 54 dialogs + screens + components in MetaRDU Industrial

---

## Executive Summary

MetaRDU Industrial has a **strong design foundation** (Sprint 12 `tokens.ts` + `DialogShell` + `DialogButton`) but **poor adoption**. Only 7 of 54 dialogs use the `DialogShell` component; the other 47 use hand-rolled boilerplate with 119 button-padding occurrences across 6 variants. Hardcoded hex colors appear in 10+ components, bypassing the token system.

**Consistency Score**: 4.2 / 10 — the foundation is right, but execution is fragmented.

---

## Audit Findings

### 1. Dialog Shell Adoption — CRITICAL

| Metric | Count | Status |
|---|---|---|
| Dialogs using `DialogShell` | 7 / 54 | 🔴 13% adoption |
| Dialogs with hand-rolled overlay/header/footer | 47 / 54 | 🔴 87% non-compliant |
| New dialogs (Sprint 12+) using `DialogShell` | 7 / 7 | ✅ 100% |
| Old dialogs (Sprint 0-11) using `DialogShell` | 0 / 47 | 🔴 0% |

**Impact**: Every old dialog has its own overlay, header, body, footer boilerplate (~40 lines each). Button placement, close-button icon, and ESC behavior vary. 47 dialogs × 40 lines = ~1,880 lines of duplicate code.

**Migration plan**: Migrate 5 dialogs per sprint (Sprint 18-27). Priority order:
1. **Volume Calc** — most-used dialog
2. **CSF Classification** — second most-used
3. **S-44 Compliance** — regulatory
4. **Dredge Audit Wizard** — revenue feature
5. **Stockpile Audit Wizard** — revenue feature

### 2. Button Padding Variants — HIGH

| Variant | Occurrences | Status |
|---|---|---|
| `px-4 py-1.5` (DialogButton standard) | 7 | ✅ Correct |
| `px-3 py-1.5` | 15 | 🟡 Off by 1px |
| `px-4 py-1.5` (inline, not DialogButton) | 45 | 🟡 Correct size, not using component |
| `px-3 py-1` | 20 | 🔴 Wrong size |
| `px-2.5 py-1.5` | 12 | 🟡 Wrong size |
| `px-2 py-1.5` | 20 | 🔴 Wrong size |

**Total**: 119 button-padding occurrences across 54 dialogs. Only 7 use `DialogButton`.

**Fix**: Replace all inline `className="rounded-md px-..."` buttons with `<DialogButton variant="...">`. This is a mechanical find-and-replace per dialog.

### 3. Hardcoded Colors — MEDIUM

10+ components use raw hex colors instead of the `colors` token from `@/lib/tokens`:

| File | Hardcoded Colors | Should Use |
|---|---|---|
| `highwall-monitoring-wizard.tsx` | `#10B981`, `#F59E0B`, `#F97316`, `#DC2626` | `colors.pass`, `colors.warn`, `colors.accent`, `colors.failDim` |
| `backscatter-mosaic-dialog.tsx` | `#1E293B` | `colors.panel` |
| `command-palette.tsx` | `#FF0000`, `#00FF00` (in heatmap) | `colors.fail`, `colors.pass` |

**Fix**: Replace all hardcoded hex with `colors.*` references. Enables the colorblind palette (Sprint 17) to work consistently.

### 4. Dialog Max-Height Variants — LOW

| Variant | Count |
|---|---|
| `max-h-[88vh]` (DialogShell standard) | 7 |
| `max-h-[85vh]` | 4 |
| `max-h-[90vh]` | 5 |
| `max-h-[92vh]` | 1 |
| `max-h-[95vh]` | 1 |

**Fix**: Standardize to `max-h-[88vh]` during DialogShell migration.

### 5. Dialog Max-Width Variants — LOW

| Variant | Count |
|---|---|
| `max-w-2xl` | 15 |
| `max-w-3xl` | 12 |
| `max-w-4xl` | 8 |
| `max-w-lg` | 5 |
| `max-w-xl` | 7 |
| `max-w-md` | 3 |

**Recommendation**: Not a problem — different dialogs legitimately need different widths. The `DialogShell` `maxWidth` prop handles this correctly.

### 6. Icon Usage Consistency — MEDIUM

| Issue | Count |
|---|---|
| Dialogs with icon in header | 47 / 54 (87%) ✅ |
| Dialogs with icon color matching domain | 12 / 54 (22%) 🔴 |
| Dialogs using `colors.accent` for icon | 30 / 54 (56%) 🟡 |

**Fix**: Use domain accent color (`domainAccent[domain].primary`) for all dialog header icons. Mining → amber, Marine → teal.

### 7. Typography Consistency — LOW

| Element | Size | Used Consistently |
|---|---|---|
| Dialog title | `text-sm font-semibold` | ✅ Yes |
| Section headers | `text-[10px] font-semibold uppercase tracking-wider` | ✅ Yes |
| Body text | `text-xs` | ✅ Yes |
| Monospace data | `font-mono text-[10px]` | ✅ Yes |
| Footer hint | `text-[10px] text-steel-gray` | ✅ Yes |

Typography is the most consistent area — the design tokens are well-established.

---

## Migration Plan

### Phase 1: Sprint 18 (5 dialogs)
1. Volume Calc Dialog → DialogShell + DialogButton
2. CSF Classification Dialog → DialogShell + DialogButton
3. S-44 Compliance Dialog → DialogShell + DialogButton
4. S-44 Certificate Dialog → DialogShell + DialogButton
5. SVP Editor Dialog → DialogShell + DialogButton

### Phase 2: Sprint 19 (5 dialogs)
6. Dredge Audit Wizard → DialogShell + DialogButton
7. Stockpile Audit Wizard → DialogShell + DialogButton
8. Blast Report Wizard → DialogShell + DialogButton
9. Highwall Monitoring Wizard → DialogShell + DialogButton + fix hardcoded colors
10. Cross-Section Profiler Wizard → DialogShell + DialogButton

### Phase 3: Sprint 20 (5 dialogs)
11. Deliverable Package Wizard → DialogShell + DialogButton
12. EOM Reconciliation Wizard → DialogShell + DialogButton
13. EOM Auditor Dialog → DialogShell + DialogButton
14. Pipeline Editor Dialog → DialogShell + DialogButton
15. Project Manager Dialog → DialogShell + DialogButton

### Phase 4: Sprint 21-27 (remaining 32 dialogs)
- Batch 5 dialogs per sprint until all 47 are migrated
- Each migration: ~30 minutes (replace boilerplate with DialogShell, replace buttons with DialogButton, replace hardcoded colors with tokens)

### Parallel: Hardcoded Color Sweep
- Replace all `#XXXXXX` in component files with `colors.*` references
- Enables the colorblind palette to work on all components
- ~2 hours of work, can be done in a single sprint

---

## Brand Identity Summary

MetaRDU Industrial's brand identity is **"Professional GIS for Mining + Marine"**:

| Element | Standard | Compliance |
|---|---|---|
| **Primary color** | Industrial Orange `#F97316` | ✅ Consistent |
| **Mining accent** | Amber `#FBBF24` | ✅ Consistent |
| **Marine accent** | Teal `#2DD4BF` | ✅ Consistent |
| **Background** | Slate-900 `#0F172A` | ✅ Consistent |
| **Panel** | Slate-800 `#1E293B` | ✅ Consistent |
| **Border** | Slate-600 `#475569` | ✅ Consistent |
| **Typography** | System UI + JetBrains Mono | ✅ Consistent |
| **Dialog chrome** | DialogShell standard | 🔴 13% adoption |
| **Buttons** | DialogButton 5 variants | 🔴 6% adoption |
| **Colorblind palette** | data-palette attribute | 🟡 Hook exists, components don't use it |

---

## Bottom Line

The design system is **well-defined** (Sprint 12 tokens + components) but **poorly adopted** (13% DialogShell, 6% DialogButton). The migration is mechanical — 47 dialogs × 30 minutes = ~24 hours of work, spread across Sprints 18-27.

The highest-ROI single action is the **hardcoded color sweep** — replace all `#XXXXXX` with `colors.*` references so the colorblind palette (Sprint 17) actually works on all components. That's 2 hours, not 24.
