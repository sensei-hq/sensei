/**
 * Identities that need you — functional e2e (user, 2026-10-29).
 *
 * The overlay that covered the screen is gone. An identity that needs you is said in
 * the main sidebar's footer while it lasts and announced once as a toast; View →
 * Identities… opens Settings · Dōjō, where every identity has its action. Unit tests
 * cover the sentence and the once-only announcement (personas.spec); this covers the
 * wiring only a real app can: the live endpoint, a clean launch, and the menu event.
 *
 * The e2e DB has no sign-ins, so nothing can have broken: a clean launch must show
 * no footer row and no toast.
 */

import { test, expect } from '../fixtures';
import { navigateToScreen, DAEMON_URL, installErrorTrap, readErrors, seedSetupComplete } from '../helpers';

test.describe('identities that need you', () => {
  test('the personas endpoint answers with the shape the app reads', async () => {
    // Against the REAL e2e daemon and its own database: a 500 here means the
    // forge-token columns did not reach sensei_e2e, which no unit test would catch.
    const res = await fetch(`${DAEMON_URL}/api/auth/personas`);
    expect(res.status).toBe(200);
    const body = (await res.json()) as {
      personas: Array<{
        label: string;
        connected: boolean;
        renewalDue: boolean;
        keychain: string | null;
        forgeToken: { state: string; expiresAt: number | null };
        action?: unknown;
      }>;
    };
    expect(Array.isArray(body.personas)).toBe(true);
    // The facts the identity state decides from — and no decision of the daemon's own.
    for (const p of body.personas) {
      expect(typeof p.label).toBe('string');
      expect(typeof p.connected).toBe('boolean');
      expect(typeof p.renewalDue).toBe('boolean');
      expect([null, 'readable', 'needsApproval', 'missing']).toContain(p.keychain);
      expect(typeof p.forgeToken.state).toBe('string');
      expect(p.action).toBeUndefined();
    }
  });

  test('a clean launch says nothing, and the menu event opens Settings · Dōjō', async ({ tauriPage }) => {
    await installErrorTrap(tauriPage);
    await seedSetupComplete(tauriPage);
    await navigateToScreen(tauriPage, '/', '[data-component="observatory-sidebar"]');

    // Nothing has broken on a fresh install: no overlay, no footer row, no toast.
    await expect(tauriPage.locator('[data-component="sign-in-overlay"]')).toHaveCount(0);
    await expect(tauriPage.locator('[data-attention]')).toHaveCount(0);
    await expect(tauriPage.locator('[data-alert-list] [data-message-root]')).toHaveCount(0);

    // View → Identities… emits exactly this from Rust; `withGlobalTauri` lets the
    // test emit it too, exercising the layout's listener without the native menu.
    await tauriPage.evaluate(`window.__TAURI__.event.emit('open-identities')`);
    await expect
      .poll(() => tauriPage.evaluate(`window.__TAURI__.webviewWindow.WebviewWindow.getByLabel('settings').then((w) => !!w)`), {
        timeout: 15_000,
      })
      .toBe(true);
    await tauriPage.evaluate(`window.__TAURI__.webviewWindow.WebviewWindow.getByLabel('settings').then((w) => w && w.close())`);

    const errs = await readErrors(tauriPage);
    expect({ ...errs, console: errs.console.filter((m) => !/favicon/i.test(m)) }).toEqual({
      console: [],
      error: [],
      rejection: [],
    });
  });
});
