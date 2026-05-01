/**
 * Boot flow test — simulates what Tauri does on launch.
 * App starts at /health, auto-advances to /setup/welcome when all gates ready.
 */

import { test, expect } from '../fixtures';

test.describe('Boot flow', () => {
  test('health page loads without errors', async ({ tauriPage }) => {
    await tauriPage.goto('/health');
    // Health page should render — even if gates aren't met, it shouldn't crash
    // Just verify the page navigated successfully
    const url = await tauriPage.url();
    expect(url).toContain('/health');
  });

  test('direct navigation to /setup/welcome works', async ({ tauriPage }) => {
    await tauriPage.goto('/setup/welcome');
    await expect(tauriPage.locator('.stage-title')).toContainText('Welcome');
  });

  test('direct navigation to /setup/preferences works', async ({ tauriPage }) => {
    await tauriPage.goto('/setup/preferences');
    await expect(tauriPage.locator('.stage-title')).toContainText('Preferences');
  });

  test('direct navigation to /setup/assistants works', async ({ tauriPage }) => {
    await tauriPage.goto('/setup/assistants');
    await expect(tauriPage.locator('.assistants')).toBeVisible();
  });

  test('direct navigation to /setup/roots works', async ({ tauriPage }) => {
    await tauriPage.goto('/setup/roots');
    await expect(tauriPage.locator('.step')).toBeVisible();
  });

  test('direct navigation to /config redirects to /setup/welcome', async ({ tauriPage }) => {
    await tauriPage.goto('/config');
    await tauriPage.waitForURL('**/setup/welcome');
  });
});
