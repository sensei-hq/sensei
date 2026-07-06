/**
 * Libraries surface — render + toggle.
 *
 * Since the wizard → Preferences arch change, Libraries lives at
 * /settings/libraries (in the observatory rail), not in the wizard.
 * The Settings surface has no Continue mechanic, so the old "toggle +
 * Continue writes setup.libraries" flow no longer applies. The toggle
 * still binds to `wizardState.libraries.libs[i].enabled` via the shared
 * LibrariesSection component; the persistence layer for that state
 * flows through the daemon's live-edit path in Settings.
 *
 * Coverage retained: render + row visibility + toggle updates the
 * data-enabled attribute (the wizardState mutation).
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

test.describe('Libraries — Settings surface', () => {
  test.beforeEach(async ({ tauriPage }) => {
    await seedHealth(tauriPage);
    await navigateTo(tauriPage, '/settings/libraries');
  });

  test('renders without error', async ({ tauriPage }) => {
    // Empty-state placeholder OR summary chips render after hydrate; the
    // branch depends on whether scan has populated libs in this session.
    const libs = await fetch(`${DAEMON_URL}/api/libs`).then(r => r.json()) as { total: number };
    const target = libs.total === 0
      ? '[data-testid="libraries-empty"]'
      : '[data-testid="libraries-summary"]';
    await expect(tauriPage.locator(target)).toBeVisible({ timeout: 10_000 });
  });

  test('row toggle flips data-enabled', async ({ tauriPage }) => {
    const libs = await fetch(`${DAEMON_URL}/api/libs`).then(r => r.json()) as { total: number; libs: Array<{ name: string }> };
    if (libs.total === 0) {
      test.skip(true, 'No libraries detected — cannot exercise toggle');
      return;
    }
    const firstLib = libs.libs[0].name;
    const row = tauriPage.locator(`[data-testid="library-row-${firstLib}"]`);
    await expect(row).toBeVisible({ timeout: 10_000 });
    await expect(row).toHaveAttribute('data-enabled', 'true', { timeout: 10_000 });
    await row.locator('button.switch').click();
    await expect(row).toHaveAttribute('data-enabled', 'false', { timeout: 5_000 });
  });
});
