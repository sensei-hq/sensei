/**
 * Settings · Roots — removing a root asks keep-or-remove (#247).
 *
 * The controller spec covers the wording and the failure paths against an
 * injected API. This covers the part it cannot: the real daemon refusing a
 * removal with no decision, and the screen changing a row ONLY after the daemon
 * says the removal happened — `keep` leaves the row with a paused chip, `remove`
 * takes it away.
 *
 * Runs against the throwaway e2e daemon and its `sensei_e2e` DB, on a root
 * pointed at a fresh temp directory, so nothing real is paused or removed.
 */

import { mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test, expect } from '../fixtures';
import { navigateTo, daemonGet, daemonPost, seedSetupComplete } from '../helpers';

async function seedHealth(tauriPage: any): Promise<void> {
  await tauriPage.evaluate(`
    (function() {
      sessionStorage.setItem('sensei:health', 'ready');
      localStorage.removeItem('sensei:setup-complete');
    })()
  `);
}

async function rootStatus(path: string): Promise<string | null> {
  const roots = await daemonGet<Array<{ path: string; status: string }>>('/api/scan/roots');
  return roots.find((r) => r.path === path)?.status ?? null;
}

test.describe('Settings · Roots · removal', () => {
  test.beforeEach(async ({ tauriPage }) => {
    test.setTimeout(180_000);
    await seedHealth(tauriPage);
    // AFTER seedHealth, which clears the local flag. Setup must be complete or
    // `hooks.ts` reroutes every page to setup, and a reset e2e DB has no
    // `setup_complete` row.
    await seedSetupComplete(tauriPage);
  });

  test('keep pauses the root, remove takes it away, and neither happens unasked', async ({ tauriPage }) => {
    const path = mkdtempSync(join(tmpdir(), 'sensei-e2e-root-'));
    await daemonPost('/api/scan/roots', { path, excluded: [] });

    await navigateTo(tauriPage, '/settings/roots');
    const remove = `button[aria-label="Remove ${path}"]`;
    await tauriPage.waitForSelector(remove, 30_000);

    // The × asks; it does not delete.
    await tauriPage.locator(remove).click();
    await tauriPage.waitForSelector('[data-testid="root-removal-question"]', 30_000);
    expect(await rootStatus(path), 'opening the panel removed nothing').not.toBeNull();

    // Keep: the row stays, marked paused, and the daemon agrees.
    await tauriPage.locator('[data-testid="root-removal-keep"]').click();
    await tauriPage.waitForSelector('[data-testid="root-paused"]', 30_000);
    expect(await rootStatus(path)).toBe('paused');

    // Remove: the row goes, and so does the daemon's root.
    await tauriPage.locator(remove).click();
    await tauriPage.waitForSelector('[data-testid="root-removal-remove"]', 30_000);
    await tauriPage.locator('[data-testid="root-removal-remove"]').click();
    await expect(tauriPage.locator(remove)).toHaveCount(0, { timeout: 30_000 });
    expect(await rootStatus(path), 'the daemon no longer has the root').toBeNull();
  });
});
