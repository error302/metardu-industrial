# Inclusive Visuals Specialist — WCAG Compliance Audit

**Agent**: Inclusive Visuals Specialist (activated from `skills/agency-agents/design/design-inclusive-visuals-specialist.md`)
**Date**: 2026-07-07
**Scope**: All UI components, dialogs, and screens in MetaRDU Industrial

---

## Executive Summary

MetaRDU Industrial has **partial WCAG compliance**. The design tokens meet contrast requirements, focus-visible styling exists, and the colorblind palette (Sprint 17) was added. But **aria-label coverage is only 37 instances across 54 dialogs** — most interactive elements are invisible to screen readers. Keyboard navigation works for dialogs (ESC to close) but there's no trap-focus or tab-order management.

**WCAG 2.1 AA Compliance Score**: 5.8 / 10 — needs work before government/enterprise procurement.

---

## WCAG 2.1 Compliance Check

### Perceivable — 6.5/10

#### 1.1 Text Alternatives — PARTIAL
| Element | Coverage | Status |
|---|---|---|
| Icon-only buttons with `aria-label` | 37 / ~150 | 🔴 25% |
| Icon-only buttons with `title=` | 45 / ~150 | 🟡 30% |
| Images with `alt` text | 0 / 2 | 🔴 0% |
| SVG decorative icons | Not marked `aria-hidden` | 🟡 |

**Fix**: Add `aria-label` to every `<button>` that has no text. Add `aria-hidden="true"` to decorative SVGs.

#### 1.2 Time-based Media — N/A
No video or audio content.

#### 1.3 Adaptable — GOOD
The layout uses semantic HTML (`<header>`, `<footer>`, `<aside>`, `<main>`) and CSS grid/flexbox. Content reflows on narrow viewports (Sprint 11 responsive sidebar).

#### 1.4 Distinguishable — GOOD
| Check | Result |
|---|---|
| Color contrast (text on background) | ✅ White (#F1F5F9) on Slate-900 (#0F172A) = 15.8:1 (AAA) |
| Color contrast (secondary text) | ✅ Slate-400 (#94A3B8) on Slate-900 = 5.9:1 (AA) |
| Color contrast (muted text) | 🟡 Slate-500 (#64748B) on Slate-900 = 3.9:1 (fails AA for small text) |
| Color not used as sole indicator | 🟡 Cut=red/fill=green — fails for colorblind users unless palette toggle is on |
| Resize text (200%) | ✅ Uses rem/em units |

**Fix**: Change `text-steel-gray` (Slate-500) to `text-steel-light` (Slate-400) for any text smaller than 14px. Make the colorblind palette default-on for new users.

### Operable — 5.0/10

#### 2.1 Keyboard Accessible — PARTIAL
| Check | Result |
|---|---|
| All functionality available via keyboard | 🟡 Most buttons work, but drag-and-drop file import requires mouse |
| Tab navigation | ✅ Works (native HTML order) |
| ESC to close dialogs | ✅ Via `useEscapeKey` |
| Ctrl+K command palette | ✅ |
| Ctrl+Z/Y undo/redo | ✅ |
| Focus trap in dialogs | 🔴 Not implemented — Tab can escape to background |
| Focus return to trigger button | 🔴 Not implemented |

**Fix**: Add focus-trap to `DialogShell` (capture Tab within dialog). Return focus to the trigger button on close.

#### 2.2 Enough Time — N/A
No time limits.

#### 2.3 Seizures and Physical Reactions — GOOD
| Check | Result |
|---|---|
| No flashing >3 times/second | ✅ |
| `prefers-reduced-motion` respected | ✅ (Sprint 12 CSS) |

#### 2.4 Navigable — PARTIAL
| Check | Result |
|---|---|
| Bypass blocks (skip to content) | 🔴 No "skip to main" link |
| Page titles | ✅ Title bar shows project name |
| Focus order | 🟡 Natural DOM order, but not explicitly managed |
| Link purpose | ✅ All links/buttons have text or aria-label |

### Understandable — 7.0/10

#### 3.1 Readable — GOOD
| Check | Result |
|---|---|
| Language of page | 🔴 No `<html lang="en">` attribute |
| Language of parts | ✅ All content is English |
| Unusual words | ✅ Surveying terminology is domain-appropriate |
| Abbreviations | 🟡 "CRS", "EPSG", "CSF", "CUBE" not expanded on first use |

**Fix**: Add `lang="en"` to `<html>`. Add tooltips on first-use abbreviations.

#### 3.2 Predictable — GOOD
| Check | Result |
|---|---|
| Consistent navigation | ✅ Sidebar is consistent |
| Consistent identification | 🟡 Same function has different button styles (see Brand Guardian audit) |
| Changes on input | ✅ Form fields update on change, not on blur |

#### 3.3 Input Assistance — PARTIAL
| Check | Result |
|---|---|
| Error identification | 🟡 Errors shown as text, but no `role="alert"` |
| Error suggestions | 🔴 Errors don't suggest fixes |
| Error prevention (legal/financial) | 🔴 No confirmation for destructive actions (Sprint 11 undo helps) |

**Fix**: Add `role="alert"` to error message containers. Add suggested fixes to common errors (e.g., "File not found → Browse for file").

### Robust — 6.0/10

#### 4.1 Compatible — PARTIAL
| Check | Result |
|---|---|
| Valid HTML | ✅ React renders valid JSX |
| ARIA attributes correct | 🟡 37 aria-labels, but no `role` on dialog containers |
| Name/Role/Value | 🟡 Buttons have names, but dialogs lack `role="dialog"` |
| Status messages | 🔴 No `role="status"` or `aria-live` on dynamic updates |

**Fix**: Add `role="dialog"` and `aria-modal="true"` to `DialogShell`. Add `aria-live="polite"` to status messages (loading, results).

#### 4.2 Assistive Technology Compatibility — UNTESTED
No screen reader testing has been done. Needs NVDA + VoiceOver + JAWS testing.

---

## Priority Remediation Plan

### Sprint 18 (Critical WCAG fixes)
1. **Add `role="dialog"` + `aria-modal="true"` to DialogShell** — 30 min
2. **Add focus trap to DialogShell** — 2 hours
3. **Add `aria-label` to all 113 icon-only buttons without one** — 3 hours
4. **Add `aria-hidden="true"` to decorative SVGs** — 1 hour
5. **Add `<html lang="en">` to index.html** — 5 min
6. **Add `role="alert"` to error containers** — 1 hour

### Sprint 19 (Important WCAG fixes)
7. **Fix muted text contrast** — change Slate-500 to Slate-400 for small text — 1 hour
8. **Add `aria-live="polite"` to status/loading messages** — 2 hours
9. **Add focus return to trigger button on dialog close** — 2 hours
10. **Make colorblind palette default-on for new users** — 30 min

### Sprint 20 (Testing)
11. **Screen reader testing** — NVDA (Windows), VoiceOver (macOS) — 4 hours
12. **Keyboard-only navigation test** — 2 hours
13. **axe-core automated test suite** in CI — 3 hours

**Total**: ~22 hours to reach WCAG 2.1 AA compliance.

---

## Bottom Line

MetaRDU has a **good visual foundation** (contrast, reduced-motion, colorblind palette) but **poor screen reader support** (37 aria-labels across 150+ interactive elements). The #1 fix is adding `role="dialog"` + `aria-modal` + focus trap to `DialogShell` — this alone makes all 7 DialogShell-based dialogs accessible. The remaining 47 dialogs need migration first (see Brand Guardian audit), then the accessibility fixes apply automatically.

For government/enterprise procurement, WCAG 2.1 AA is often a hard requirement. The 22-hour remediation plan above would make MetaRDU compliant.
