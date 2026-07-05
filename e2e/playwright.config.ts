import { defineConfig, devices } from '@playwright/test';

/**
 * Playwright configuration for MetaRDU Industrial E2E tests.
 *
 * These tests run against the Vite dev server in browser mode
 * (no Tauri shell required). They verify:
 *   - Splash screen renders
 *   - Module loading screen renders
 *   - Onboarding flow works
 *   - Workspace shell renders with map canvas
 *   - All 33 dialogs can be opened + closed via keyboard shortcuts
 *   - Settings dialog can change theme
 *
 * For native Tauri testing (testing IPC commands), see
 * e2e/TAURI_TESTING.md.
 *
 * Usage:
 *   cd e2e && npm install && npx playwright install chromium
 *   npm test                     # headless
 *   npm run test:headed          # with browser window
 *   npm run test:ui              # interactive UI mode
 */

export default defineConfig({
  testDir: './tests',
  fullyParallel: false, // Sequential — the app has global state
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  workers: 1, // Single worker — app state is shared
  reporter: [
    ['html', { outputFolder: 'playwright-report' }],
    ['list'],
  ],
  use: {
    baseURL: 'http://localhost:1420',
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
    video: 'retain-on-failure',
    actionTimeout: 10_000,
    navigationTimeout: 15_000,
  },
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
  ],
  webServer: {
    command: 'cd .. && npm run dev',
    url: 'http://localhost:1420',
    reuseExistingServer: !process.env.CI,
    timeout: 60_000,
  },
});
