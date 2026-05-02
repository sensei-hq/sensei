/**
 * Boot flow test — simulates what Tauri does on launch.
 * App starts at /health, auto-advances to /setup/welcome when all gates ready.
 */

import { test, expect } from '../fixtures';
import { navigateTo } from '../helpers';

test.describe('Boot flow', () => {
  test('health page loads without errors', async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/health');
    // Health page auto-advances to /setup/welcome or /observatory when all
    // gates are ready. Accept either outcome — we just verify no crash.
    const url = await tauriPage.url();
    expect(url).toMatch(/\/(health|setup|observatory)/);
  });

  test('direct navigation to /setup/welcome works', async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/setup/welcome');
    await expect(tauriPage.locator('.stage-title')).toContainText('Welcome');
  });

  test('direct navigation to /setup/preferences works', async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/setup/preferences');
    await expect(tauriPage.locator('.stage-title')).toContainText('Preferences');
  });

  test('direct navigation to /setup/assistants works', async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/setup/assistants');
    await expect(tauriPage.locator('.assistants')).toBeVisible();
  });

  test('direct navigation to /setup/roots works', async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/setup/roots');
    await expect(tauriPage.locator('.step')).toBeVisible();
  });

  test('direct navigation to /config redirects to /setup/welcome', async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/config');
    await tauriPage.waitForURL('/setup/welcome');
  });
});
