/**
 * Libraries stage — render + toggle + commit persistence.
 *
 * The daemon only knows about libs after a scan, and Flow B's corpus
 * (a single git repo with `lodash` in package.json) populates exactly
 * one library. We test against the live daemon's actual state:
 *
 *   • Empty case  — no libs detected → placeholder shows
 *   • Hydration   — page loads without error and renders summary chips
 *   • Persistence — toggling a lib + Continue writes the wrapped/disabled
 *     split to `setup.libraries`
 */

import { test, expect } from '../fixtures';
import { navigateTo, DAEMON_URL } from '../helpers';

async function seedHealth(tauriPage: any): Promise<void> {
  await tauriPage.evaluate(`
    (function() {
      sessionStorage.setItem('sensei:health', 'ready');
      localStorage.removeItem('sensei:setup-complete');
    })()
  `);
}

async function resetSetupKeys(): Promise<void> {
  const keys = [
    'setup.welcome', 'setup.preferences', 'setup.assistants',
    'setup.roots', 'setup.scan', 'setup.libraries', 'setup_complete',
  ];
  for (const k of keys) {
    await fetch(`${DAEMON_URL}/api/config/${k}`, { method: 'DELETE' });
  }
}

async function waitForPath(tauriPage: any, expected: string, timeoutMs = 20_000): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const p = await tauriPage.evaluate(`window.location.pathname`).catch(() => null);
    if (p === expected) return;
    await new Promise(r => setTimeout(r, 100));
  }
  throw new Error(`Timed out waiting for path ${expected}`);
}

test.describe('Libraries stage', () => {
  test.beforeEach(async ({ tauriPage }) => {
    await resetSetupKeys();
    await seedHealth(tauriPage);
    await navigateTo(tauriPage, '/logs');
    await navigateTo(tauriPage, '/setup/libraries');
  });

  test('renders without error', async ({ tauriPage }) => {
    // Either the empty-state placeholder OR the summary chips should show
    // after hydrate. The exact branch depends on whether scan has populated
    // libs in this test session.
    const libs = await fetch(`${DAEMON_URL}/api/libs`).then(r => r.json()) as { total: number };
    const target = libs.total === 0
      ? '[data-testid="libraries-empty"]'
      : '[data-testid="libraries-summary"]';
    await expect(tauriPage.locator(target)).toBeVisible({ timeout: 10_000 });
  });

  test('toggles persist via setup.libraries config on Continue', async ({ tauriPage }) => {
    const libs = await fetch(`${DAEMON_URL}/api/libs`).then(r => r.json()) as { total: number; libs: Array<{ name: string }> };
    if (libs.total === 0) {
      test.skip(true, 'No libraries detected — cannot exercise toggle flow');
      return;
    }

    // Wait for the first library row, then flip its switch off.
    const firstLib = libs.libs[0].name;
    const row = tauriPage.locator(`[data-testid="library-row-${firstLib}"]`);
    await expect(row).toBeVisible({ timeout: 10_000 });
    await expect(row).toHaveAttribute('data-enabled', 'true', { timeout: 10_000 });
    await row.locator('button.switch').click();
    await expect(row).toHaveAttribute('data-enabled', 'false', { timeout: 5_000 });

    await tauriPage.locator('.btn-primary').click();
    await waitForPath(tauriPage, '/setup/instruments', 20_000);

    // Daemon persisted the toggle via JSON-string in setup.libraries.
    const cfg = await fetch(`${DAEMON_URL}/api/config`).then(r => r.json()) as Record<string, string>;
    const stored = JSON.parse(cfg['setup.libraries']) as { wrapped: string[]; disabled: string[] };
    expect(stored.disabled).toContain(firstLib);
    expect(stored.wrapped).not.toContain(firstLib);
  });
});
