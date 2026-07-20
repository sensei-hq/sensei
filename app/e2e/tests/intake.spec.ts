/**
 * Intake — the front door (Operating-model Phase 2).
 *
 * Verifies the new /intake screen end-to-end in the real built Tauri app
 * against a fresh e2e daemon: the route renders, the rail's "Intake" anchor
 * lands on it, and the freeform → recommend → confirm flow round-trips through
 * the daemon (classify + recommend + record).
 *
 * Seed-agnostic: the throwaway sensei_e2e DB has the playbook DDL but no
 * imported catalog/rules, so `recommend` returns a defaulted playbook (raw
 * name, no title/tone) rather than a seeded one. The flow is identical — this
 * spec asserts the plumbing (routes live, card renders, run records), not the
 * specific playbook copy (that's the seeded-DB visual smoke test).
 */

import { test, expect } from '../fixtures';
import { navigateTo, navigateToScreen, daemonGet, daemonPost, DAEMON_URL } from '../helpers';

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
  return (await tauriPage.evaluate(`window.location.pathname`)) as string;
}

test.describe('Intake — front door', () => {
  test.beforeEach(async ({ tauriPage }) => {
    await seedSetupComplete(tauriPage);
    await navigateTo(tauriPage, '/logs'); // reset the SPA before navigating to the target
  });

  test('direct navigation renders the intake screen', async ({ tauriPage }) => {
    await navigateToScreen(tauriPage, '/intake', '[data-testid="intake-input"]');
    expect(await pathname(tauriPage)).toBe('/intake');
    await expect(tauriPage.locator('[data-testid="intake-input"]')).toBeVisible();
    await expect(tauriPage.locator('[data-testid="intake-recommend"]')).toBeVisible();
  });

  test('the rail "Intake" anchor navigates to /intake', async ({ tauriPage }) => {
    await navigateToScreen(tauriPage, '/', 'a[href="/intake"]');
    await tauriPage.locator('a[href="/intake"]').first().click();
    await new Promise((r) => setTimeout(r, 800));
    expect(await pathname(tauriPage)).toBe('/intake');
  });

  test('freeform → recommend → confirm records a run', async ({ tauriPage }) => {
    await navigateToScreen(tauriPage, '/intake', '[data-testid="intake-input"]');

    await tauriPage.locator('[data-testid="intake-input"]').fill('fix the null deref when the token refreshes');
    await tauriPage.locator('[data-testid="intake-recommend"]').click();

    // Classify (gateway, heuristic fallback) + recommend can take a few seconds.
    await expect(tauriPage.locator('[data-testid="intake-card"]')).toBeVisible({ timeout: 30_000 });
    await expect(tauriPage.locator('[data-testid="intake-playbook-title"]')).not.toBeEmpty();
    await expect(tauriPage.locator('[data-testid="intake-axes"]')).toBeVisible();

    await tauriPage.locator('[data-testid="intake-confirm"]').click();
    await expect(tauriPage.locator('[data-testid="intake-recorded"]')).toBeVisible({ timeout: 15_000 });
  });

  test('daemon exposes the front-door endpoints (guide + recommend with axes)', async () => {
    const guide = await daemonGet<Record<string, unknown>>('/api/playbook/guide');
    expect(guide).toHaveProperty('playbooks');
    expect(guide).toHaveProperty('axes');
    expect(guide).toHaveProperty('frame');

    const rec = await daemonPost<Record<string, unknown>>('/api/playbook/recommend', {
      chunk: 'add a small feature flag to gate the new panel',
      preview: true,
    });
    // The recommendation carries the chosen playbook AND the classified axes
    // (added for the app form to display + drive the confirm leg).
    expect(rec).toHaveProperty('playbook');
    expect(rec).toHaveProperty('lifecycle');
    expect(rec).toHaveProperty('intent');
    expect(rec).toHaveProperty('risk');
    expect(rec).toHaveProperty('auto_select');
  });
});
