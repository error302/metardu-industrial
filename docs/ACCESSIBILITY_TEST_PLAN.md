# Testing Accessibility Auditor — axe-core Test Plan

**Agent**: Testing Accessibility Auditor (activated from `skills/agency-agents/testing/testing-accessibility-auditor.md`)
**Date**: 2026-07-07
**Scope**: Automated accessibility testing plan for MetaRDU Industrial

---

## Executive Summary

MetaRDU Industrial needs automated accessibility testing in CI to catch WCAG violations before they ship. This document defines the test plan using `axe-core` (the industry-standard accessibility testing library) integrated into the existing Playwright E2E test suite.

**Current state**: 0 automated accessibility tests.
**Target state**: axe-core runs on every dialog + screen in CI, fails on WCAG AA violations.

---

## Test Plan

### 1. Integration with Playwright E2E Tests

The existing `e2e/` directory has Playwright tests. axe-core integrates as a Playwright assertion:

```typescript
// e2e/tests/accessibility.spec.ts
import { test, expect } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

test.describe('Accessibility audits', () => {
  test('workspace shell has no WCAG violations', async ({ page }) => {
    await page.goto('/');
    const results = await new AxeBuilder({ page })
      .withTags(['wcag2a', 'wcag2aa'])
      .analyze();
    expect(results.violations).toEqual([]);
  });

  // Test each dialog by opening it via the command palette
  test('volume calc dialog has no violations', async ({ page }) => {
    await page.goto('/');
    await page.keyboard.press('Control+k');
    await page.fill('input[placeholder*="Search"]', 'volume');
    await page.keyboard.press('Enter');
    const results = await new AxeBuilder({ page })
      .withTags(['wcag2a', 'wcag2aa'])
      .analyze();
    expect(results.violations).toEqual([]);
  });
});
```

### 2. Test Coverage Matrix

| Screen/Dialog | Test ID | Priority | Status |
|---|---|---|---|
| Workspace shell (map page) | `workspace-shell` | High | Pending |
| Settings dialog | `settings-dialog` | High | Pending |
| Volume calc dialog | `volume-calc` | High | Pending |
| CSF classification | `csf-dialog` | High | Pending |
| S-44 compliance | `s44-dialog` | High | Pending |
| EOM auditor | `eom-auditor` | High | Pending |
| Stockpile audit wizard | `stockpile-audit` | High | Pending |
| Dredge audit wizard | `dredge-audit` | High | Pending |
| Command palette | `command-palette` | Medium | Pending |
| Splash screen | `splash` | Low | Pending |
| Module loading screen | `module-loading` | Low | Pending |
| All DialogShell-based dialogs | `dialog-shell-*` | Medium | Pending |

### 3. CI Integration

Add to `.github/workflows/e2e.yml`:
```yaml
- name: Accessibility audit
  run: npx playwright test e2e/tests/accessibility.spec.ts
  env:
    CI: true
```

The test fails the CI build if any WCAG AA violations are found.

### 4. Known Issues to Fix Before Tests Pass

Based on the WCAG audit (Sprint 18):

| Issue | axe-core Rule | Fix |
|---|---|---|
| Dialogs without `role="dialog"` | `aria-dialog-name` | ✅ Fixed in Sprint 19 |
| No focus trap | `tabindex` | ✅ Fixed in Sprint 19 |
| Missing `aria-label` on buttons | `button-name` | Pending — 113 buttons |
| Missing `<html lang="en">` | `html-has-lang` | Pending — 5 min fix |
| Color contrast (muted text) | `color-contrast` | Pending — change Slate-500 to Slate-400 |

### 5. Implementation Steps

1. Install `@axe-core/playwright` — `npm install -D @axe-core/playwright`
2. Create `e2e/tests/accessibility.spec.ts` with the test matrix above
3. Fix the 5 known issues (lang, contrast, remaining aria-labels)
4. Run the tests — they should pass
5. Add to CI — fails on any new violations

**Effort**: 4 hours (install + write tests + fix known issues + CI integration)

---

## Bottom Line

axe-core in CI is the safety net that prevents accessibility regressions. The Sprint 19 DialogShell fixes (role=dialog, focus trap, aria-modal) are the prerequisite — without them, every DialogShell-based dialog would fail the audit. With the fixes in place, the test suite should pass after fixing the remaining `<html lang>` and contrast issues.

This is a one-time 4-hour investment that pays off every time a developer adds a new dialog or component — the CI will catch accessibility violations automatically.
