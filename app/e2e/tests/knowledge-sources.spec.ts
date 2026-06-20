/**
 * Knowledge sources (#27) — reproduces the reported "clicking Sources bounces
 * back to home / looks like settings". Verifies the route renders and the
 * sidebar nav item lands on it, in the real built Tauri app.
 */

import { test, expect } from '../fixtures';
import { navigateTo, DAEMON_URL } from '../helpers';

/** Mark setup complete so the observatory shell is reachable (not rerouted to setup). */
async function seedSetupComplete(tauriPage: any): Promise<void> {
  await fetch(`${DAEMON_URL}/api/config`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ setup_complete: '1' }),
  });
  await tauriPage.evaluate(`
    (function() {
      var s = window.__sensei_state__;
      if (s && s.appState) {
        s.appState.config = Object.assign({}, s.appState.config, { setup_complete: '1' });
        s.appState.loaded = true;
      }
    })()
  `);
}

async function pathname(tauriPage: any): Promise<string> {
  return await tauriPage.evaluate(`window.location.pathname`);
}

test.describe('Knowledge sources (#27)', () => {
  test.beforeEach(async ({ tauriPage }) => {
    await seedSetupComplete(tauriPage);
    await navigateTo(tauriPage, '/logs'); // reset the SPA before navigating to the target
  });

  test('direct navigation renders the screen (no bounce to home)', async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/knowledge-sources');
    expect(await pathname(tauriPage)).toBe('/knowledge-sources');
    await expect(
      tauriPage.locator('[data-component="page-header"], h1, h2').filter({ hasText: 'Knowledge sources' }),
    ).toBeVisible();
  });

  test('sidebar "Sources" item navigates to /knowledge-sources', async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/'); // observatory home
    expect(await pathname(tauriPage)).toBe('/');
    await tauriPage.locator('a[href="/knowledge-sources"]').click();
    await new Promise((r) => setTimeout(r, 800));
    expect(await pathname(tauriPage)).toBe('/knowledge-sources');
  });
});
