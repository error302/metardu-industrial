import { test, expect } from '@playwright/test';

/**
 * Workspace shell tests — verify the main workspace renders correctly
 * after boot.
 *
 * These tests assume the app has completed onboarding (we skip it by
 * setting localStorage flags before navigation).
 */

test.beforeEach(async ({ page }) => {
  // Skip onboarding by setting the localStorage flag
  await page.addInitScript(() => {
    localStorage.setItem('metardu.onboarded', '1');
    localStorage.setItem('metardu.settings', JSON.stringify({
      defaultDomain: 'both',
      defaultEpsg: 'EPSG:4326',
      density: 'comfortable',
      reducedMotion: false,
    }));
    localStorage.setItem('metardu.theme', 'dark');
  });
});

test.describe('Workspace shell', () => {
  test('map canvas renders', async ({ page }) => {
    await page.goto('/');
    // Skip splash + modules by waiting for workspace
    await page.waitForTimeout(5_000);
    // The OpenLayers map should render
    const mapDiv = page.locator('.ol-viewport');
    await expect(mapDiv).toBeVisible({ timeout: 15_000 });
  });

  test('title bar shows app name', async ({ page }) => {
    await page.goto('/');
    await page.waitForTimeout(5_000);
    // The title bar should show the brand name
    await expect(page.locator('text=MetaRDU Industrial').first()).toBeVisible({
      timeout: 15_000,
    });
  });

  test('status bar shows CRS + UTC clock', async ({ page }) => {
    await page.goto('/');
    await page.waitForTimeout(5_000);
    // The status bar should show EPSG
    await expect(page.locator('text=EPSG:4326')).toBeVisible({
      timeout: 15_000,
    });
  });

  test('empty state hint shows when no files loaded', async ({ page }) => {
    await page.goto('/');
    await page.waitForTimeout(5_000);
    // The empty-state hint should show
    await expect(page.locator('text=No survey loaded')).toBeVisible({
      timeout: 15_000,
    });
  });

  test('shows the survey workflow rail', async ({ page }) => {
    await page.goto('/');
    await page.waitForTimeout(5_000);

    const workflow = page.getByRole('navigation', { name: 'Survey workflow' });
    await expect(workflow).toBeVisible({ timeout: 15_000 });
    await expect(workflow.getByRole('button', { name: 'Ingest', exact: true })).toBeVisible();
    await expect(workflow.getByRole('button', { name: 'Deliver', exact: true })).toBeVisible();
  });

  test('reports browser-only survey drops as uninspected', async ({ page }) => {
    await page.goto('/');
    const brief = page.getByRole('complementary', { name: 'Survey brief' });
    await expect(brief).toBeVisible({ timeout: 20_000 });
    const transfer = await page.evaluateHandle(() => {
      const data = new DataTransfer();
      data.items.add(new File(['not a survey'], 'example.las', { type: 'application/octet-stream' }));
      return data;
    });
    await page.locator('main').dispatchEvent('dragover', { dataTransfer: transfer });
    await page.locator('main').dispatchEvent('drop', { dataTransfer: transfer });
    await expect(brief.getByText('Needs review')).toBeVisible();
    await expect(brief.getByText('Survey file inspection requires the native desktop application.')).toBeVisible();
    await expect(brief.getByText('0/1 loaded')).toBeVisible();
  });
});
