import { test, expect } from '@playwright/test';

/**
 * Boot sequence tests — verify the splash → modules → onboarding →
 * workspace flow renders without errors.
 *
 * These tests run in browser mode (no Tauri shell). The app detects
 * this via `isTauri()` and uses browser-mode stubs for IPC calls.
 */

test.describe('Boot sequence', () => {
  test('splash screen renders with brand name', async ({ page }) => {
    await page.goto('/');
    // The splash screen should show the app name
    await expect(page.locator('text=METARDU INDUSTRIAL')).toBeVisible({
      timeout: 10_000,
    });
    // Version + build should be visible
    await expect(page.locator('text=v0.1.0')).toBeVisible();
  });

  test('splash screen progresses to module loading', async ({ page }) => {
    await page.goto('/');
    // Wait for splash to finish (it runs for ~1.5s)
    await page.waitForTimeout(3_000);
    // Module loading screen should show "MODULE INITIALIZATION"
    await expect(page.locator('text=MODULE INITIALIZATION')).toBeVisible({
      timeout: 10_000,
    });
  });

  test('module loading shows progress ring', async ({ page }) => {
    await page.goto('/');
    await page.waitForTimeout(3_000);
    // The header should have a progress indicator
    const header = page.locator('header');
    await expect(header).toBeVisible();
    // Module list should populate
    await expect(page.locator('text=Initializing processing core')).toBeVisible({
      timeout: 10_000,
    });
  });

  test('can skip to workspace via Skip button', async ({ page }) => {
    await page.goto('/');
    await page.waitForTimeout(3_000);
    // Click the Skip button
    const skipButton = page.locator('text=Skip');
    await skipButton.click();
    // Should land on either onboarding or workspace
    await page.waitForTimeout(2_000);
    // The workspace should have the OpenLayers map
    const mapDiv = page.locator('.ol-viewport');
    await expect(mapDiv).toBeVisible({ timeout: 10_000 });
  });
});
