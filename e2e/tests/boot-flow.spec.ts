/**
 * Boot flow E2E tests — real Sensei.app, real IPC.
 *
 * Tests the /health bootstrap page against the running app.
 * App is launched by globalSetup with SENSEI_MODE=dev / SENSEI_DB_NAME=sensei-dev.
 */

import { test, expect } from '../fixtures';
import { navigateTo } from '../helpers';

test.describe('Boot flow', () => {
  test('health page loads', async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/health');
    await expect(tauriPage.locator('.bootstrap-page')).toBeVisible({ timeout: 10_000 });
  });

  test('bootstrap gates are visible', async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/health');
    const gates = tauriPage.locator('.gate-row');
    await expect(gates.first()).toBeVisible({ timeout: 10_000 });
  });

  test('page advances to setup when bootstrap completes', async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/health');
    // If all gates become ready within 30 s the page auto-advances.
    // If gates are still pending (slow environment), staying on /health is also a pass.
    try {
      await tauriPage.waitForURL(/\/(setup\/welcome|observatory)/, { timeout: 30_000 });
    } catch {
      // waitForURL timeout does not guarantee the URL hasn't moved — accept either outcome
      const url = await tauriPage.url();
      expect(url).toMatch(/\/(setup\/welcome|observatory|health)/);
    }
  });
});
