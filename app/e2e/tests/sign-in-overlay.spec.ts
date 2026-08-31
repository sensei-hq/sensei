/**
 * Sign-in overlay — functional e2e.
 *
 * Unit tests cover the standing reduction, the per-row action, the one-attempt
 * guard and the rendering (personas.spec, SignInOverlay.spec, forge-auth.spec).
 * What they cannot cover is the thing that actually went wrong in use: "I don't
 * see it in the launched app." That is a WIRING question — the menu event, the
 * layout's listener, the real endpoint — and only a real app can answer it.
 *
 * So this spec covers:
 *   1. `GET /api/auth/personas` answers on the live e2e daemon with the wire
 *      shape the app expects (proves the handler + the `forge_token_*` columns
 *      exist in sensei_e2e, not just sensei_test).
 *   2. The overlay is ABSENT on a clean launch. That is the designed behaviour
 *      and the source of the confusion: it opens itself only when a CONNECTED
 *      identity has died, never for the never-connected personas sensei infers
 *      from commit authorship.
 *   3. Emitting `open-identities` — what View → Identities… does — opens it.
 *      This is the entry point that has to work when nothing is broken.
 *   4. It closes again, so it cannot trap the user.
 *
 * The e2e DB is a throwaway with no sign-ins, so every persona reads
 * `connect` at most. Asserting the overlay opens and renders an honest state is
 * the reachable claim; a live OAuth round trip is not.
 */

import { test, expect } from '../fixtures';
import { navigateTo, DAEMON_URL, installErrorTrap, readErrors } from '../helpers';

/** Mark setup complete so the observatory shell is reachable. */
async function seedSetupComplete(tauriPage: any): Promise<void> {
  await fetch(`${DAEMON_URL}/api/config`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ setup_complete: '1' })
  });
  await tauriPage.evaluate(`
    (function() {
      try { localStorage.setItem('sensei:setup-complete', '1'); } catch (e) { /* shim */ }
      var s = window.__sensei_state__;
      if (s && s.appState) {
        s.appState.config = Object.assign({}, s.appState.config, { setup_complete: '1' });
        s.appState.loaded = true;
      }
    })()
  `);
}

const OVERLAY = '[data-component="sign-in-overlay"]';

test.describe('sign-in overlay', () => {
  test('the personas endpoint answers with the shape the app reads', async () => {
    // Against the REAL e2e daemon and its own database. A 500 here means the
    // `forge_token_state` / `forge_token_expires_at` columns did not reach
    // sensei_e2e, which no unit test would catch.
    const res = await fetch(`${DAEMON_URL}/api/auth/personas`);
    expect(res.status).toBe(200);
    const body = (await res.json()) as {
      personas: Array<{ label: string; connected: boolean; action: string }>;
    };
    expect(Array.isArray(body.personas)).toBe(true);
    // Every row carries the fields the list renders, and an action the UI knows.
    for (const p of body.personas) {
      expect(typeof p.label).toBe('string');
      expect(typeof p.connected).toBe('boolean');
      expect(['connect', 'signIn', 'renew', 'none']).toContain(p.action);
    }
  });

  test('stays shut on a clean launch, opens from the menu event, and closes', async ({
    tauriPage
  }) => {
    await installErrorTrap(tauriPage);
    await seedSetupComplete(tauriPage);
    await navigateTo(tauriPage, '/');

    // 1. ABSENT. Nothing is broken on a fresh e2e install: no persona has ever
    //    signed in, so none can have DIED. An overlay here would be the
    //    every-launch nuisance the auto-open rule exists to prevent.
    await expect(tauriPage.locator(OVERLAY)).toHaveCount(0);

    // 2. The menu event. `View → Identities…` emits exactly this from Rust; the
    //    frontend can emit it too because `withGlobalTauri` is on, which lets
    //    this test exercise the layout's listener without driving a native menu.
    await tauriPage.evaluate(`window.__TAURI__.event.emit('open-identities')`);
    await expect(tauriPage.locator(OVERLAY)).toHaveCount(1);

    // It is announced as a modal dialog with a name, not an anonymous div.
    const dialog = tauriPage.locator(OVERLAY);
    await expect(dialog).toHaveAttribute('role', 'dialog');
    await expect(dialog).toHaveAttribute('aria-modal', 'true');

    // 3. It renders exactly ONE honest state: the loading line, the empty
    //    message, an error, or a list of rows. Never "no identities" over data
    //    it has not read.
    const states = await tauriPage.evaluate(`
      (function() {
        var root = document.querySelector('${OVERLAY}');
        return {
          loading: !!root.querySelector('[data-component="sign-in-loading"]'),
          error: !!root.querySelector('[role="alert"]'),
          rows: root.querySelectorAll('[data-persona]').length,
          text: root.textContent || ''
        };
      })()
    `);
    const empty = /no identities yet/i.test(states.text);
    const shown = [states.loading, states.error, states.rows > 0, empty].filter(Boolean);
    expect(shown, JSON.stringify(states)).toHaveLength(1);

    // 4. Escape closes it. A modal that cannot be dismissed is worse than one
    //    that never opened.
    await tauriPage.keyboard.press('Escape');
    await expect(tauriPage.locator(OVERLAY)).toHaveCount(0);

    // No console.error, window error, or unhandled rejection from any of it.
    const errs = await readErrors(tauriPage);
    expect({ ...errs, console: errs.console.filter((m) => !/favicon/i.test(m)) }).toEqual({
      console: [],
      error: [],
      rejection: []
    });
  });
});
