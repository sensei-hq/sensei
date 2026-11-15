/**
 * Settings · Dōjō connection — identities first, then each one's dōjōs and what
 * each has filed (2026-11-03 revamp; failures render through the shared Notice).
 *
 * The unit tests cover the derivations against mocks. What only an e2e proves is
 * that both halves are wired to the real daemon, and — the property this screen
 * exists for — that "nothing has synced yet" and "the read failed" look different.
 * A sync surface that reports health when its own read broke is the most
 * expensive lie available here.
 *
 * Assertions are cross-checked against a live read of the same endpoints rather
 * than literals, so they test wiring and not a fixture.
 *
 * ## Exercising the POPULATED branches
 *
 * A fresh e2e database has no sync rows and no personas, so the branches below
 * take the honest-empty path — which is itself the property most worth pinning,
 * but it is not full coverage. `sensei.sync_state` has no foreign keys, so the
 * populated path can be exercised directly and was, on 2026-08-31:
 *
 * ```sql
 * INSERT INTO sensei.sync_state
 *   (entity, entity_key, direction, state, last_error, attempted_at, synced_at)
 * VALUES
 *  ('repository_metric','github.com/acme/ok','push','synced',NULL, now(), now()),
 *  ('repository_metric','github.com/acme/broken','push','error',
 *   'the dojo refused: 402', now(), now() - interval '11 days'),
 *  ('repository','github.com/acme/private','push','skipped',
 *   'repository is private', now(), NULL);
 * ```
 *
 * With rows like those (written by the signed-in identity, `persona` set), the
 * repository rows render under its dōjōs: a failing one carries a FAULT Notice and
 * a skipped one a REFUSAL Notice — never the other way round. The rows are NOT left behind — undeclared fixture state is the
 * staleness problem #142 is about, and a test passing on data nobody declared is
 * worse than one that skips.
 */

import { test, expect, type TauriPageLike } from '../fixtures';
import { navigateToScreen, DAEMON_URL } from '../helpers';

interface SyncResponse {
  count: number;
  counts: Record<string, number>;
  entities: { entity: string; entity_key: string; state: string; persona: string | null }[];
}
interface PersonasResponse {
  personas: { label: string; connected: boolean; sessionSlot: string | null; isDefault?: boolean }[];
}

/** A live read with the status checked BEFORE the body is parsed — `r.json()` on
 *  a bare-status error response throws an unhelpful JSON syntax error. */
async function live<T>(path: string): Promise<T> {
  const r = await fetch(`${DAEMON_URL}${path}`);
  expect(r.ok, `GET ${path} returned ${r.status}`).toBe(true);
  return (await r.json()) as T;
}

/** Past the health and setup gates. Setup completion is DAEMON state, not just a
 *  localStorage flag — see settings-metrics.spec.ts. */
async function seedSetupComplete(tauriPage: TauriPageLike): Promise<void> {
  await fetch(`${DAEMON_URL}/api/config`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ setup_complete: '1' }),
  });
  await tauriPage.evaluate(`
    (function() {
      sessionStorage.setItem('sensei:health', 'ready');
      try { localStorage.setItem('sensei:setup-complete', '1'); } catch (e) { /* shim */ }
      var s = window.__sensei_state__;
      if (s && s.appState) {
        s.appState.config = Object.assign({}, s.appState.config, { setup_complete: '1' });
        s.appState.loaded = true;
      }
    })()
  `);
}

test.describe('Settings · Dōjō connection', () => {
  test.beforeEach(async ({ tauriPage }) => {
    test.setTimeout(180_000);
    await seedSetupComplete(tauriPage);
    await seedSetupComplete(tauriPage);
    await navigateToScreen(tauriPage, '/settings/dojo', '[data-screen="settings-dojo"]');
  });

  test('reads both halves from the daemon, and a skip never reads as a failure', async ({ tauriPage }) => {
    const sync = await live<SyncResponse>('/api/dojo/sync-state');
    await live('/api/dojo/memberships');

    // Both reads succeeded, so no load-failure Notice may be showing.
    await expect(tauriPage.locator('[data-load-notices]')).toHaveCount(0);

    // Each repository row the page shows carries the tone its state means.
    const tones = (await tauriPage.evaluate(
      `Array.from(document.querySelectorAll('[data-repository]')).map(el => [
         el.getAttribute('data-state'),
         el.querySelector('[data-component="notice"]')?.getAttribute('data-tone') ?? null,
       ])`,
    )) as [string, string | null][];
    for (const [state, tone] of tones) {
      if (state === 'skipped') expect(tone, 'a deliberate skip is a refusal, not a fault').toBe('refusal');
      if (state === 'error') expect(tone, 'a failing repository says so').toBe('fault');
      if (state === 'synced') expect(tone).toBeNull();
    }
    // Every row shown is a repository verdict the daemon reported.
    const keys = new Set(sync.entities.filter((e) => e.entity === 'repository').map((e) => e.entity_key));
    const shown = (await tauriPage.evaluate(
      `Array.from(document.querySelectorAll('[data-repository]')).map(el => el.getAttribute('data-repository'))`,
    )) as string[];
    for (const k of shown) expect(keys.has(k), k).toBe(true);
  });

  test('shows every identity the daemon knows, with a standing line each, and no actions on the cards', async ({ tauriPage }) => {
    const { personas } = await live<PersonasResponse>('/api/auth/personas');

    // A failed registry read must NOT render as "no identities" — that phrasing
    // invites connecting one, which is the wrong action against a registry that
    // is merely unreachable.
    await expect(tauriPage.locator('[data-identities-notice]')).toHaveCount(0);

    if (personas.length === 0) {
      await expect(tauriPage.locator('[data-personas-empty]')).toBeVisible({ timeout: 15_000 });
      return;
    }

    await expect(tauriPage.locator('[data-identity]')).toHaveCount(personas.length, { timeout: 15_000 });
    for (const p of personas) {
      await expect(tauriPage.locator(`[data-identity="${p.label}"] [data-standing]`)).not.toHaveText('');
    }
    // Every action for the selected identity is in its header — one place to act.
    await expect(tauriPage.locator('[data-identity] [data-action]')).toHaveCount(0);
    await expect(tauriPage.locator('[data-identity-header]')).toBeVisible();
  });
});
