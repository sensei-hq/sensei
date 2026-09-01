/**
 * Settings · Dōjō — the credential standing and what has been agreed.
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
 * With those three rows all assertions here run and pass: the failing row renders
 * `failing · last agreed <date>` and the skipped row renders `skipped` without
 * `failing`. The rows are NOT left behind — undeclared fixture state is the
 * staleness problem #142 is about, and a test passing on data nobody declared is
 * worse than one that skips.
 */

import { test, expect } from '../fixtures';
import { navigateToScreen, DAEMON_URL } from '../helpers';

interface SyncResponse {
  count: number;
  counts: Record<string, number>;
  entities: { entity: string; entity_key: string; state: string }[];
}
interface PersonasResponse {
  personas: { label: string; connected: boolean }[];
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
async function seedSetupComplete(tauriPage: any): Promise<void> {
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

test.describe('Settings · Dōjō', () => {
  test.beforeEach(async ({ tauriPage }) => {
    test.setTimeout(180_000);
    await seedSetupComplete(tauriPage);
    await navigateToScreen(tauriPage, '/settings/dojo', '[data-screen="settings-dojo"]');
  });

  test('shows the sync list against the daemon, and empty is not an error', async ({
    tauriPage,
  }) => {
    const sync = await live<SyncResponse>('/api/dojo/sync-state');

    // The load succeeded, so no error state may be showing. `data-screen-state`
    // is hyphenated — the attribute ScreenState actually emits.
    await expect(tauriPage.locator('[data-screen-state="error"]')).toHaveCount(0);

    if (sync.count === 0) {
      // Honest-empty in words. "all agreed" over zero entities would report a
      // healthy sync on an install that has never synced anything.
      await expect(tauriPage.locator('[data-empty]')).toBeVisible({ timeout: 15_000 });
      await expect(tauriPage.locator('[data-sync-rows]')).toHaveCount(0);
      return;
    }

    const rows = tauriPage.locator('[data-sync-rows] [data-entity]');
    await expect(rows).toHaveCount(sync.count, { timeout: 15_000 });

    // Every row's state came from the daemon — read through `evaluate`, which is
    // this harness's way (its locator wrapper has no `evaluateAll`).
    const states = (await tauriPage.evaluate(
      `Array.from(document.querySelectorAll('[data-sync-rows] [data-entity]'))
         .map(el => el.getAttribute('data-state'))`,
    )) as string[];
    expect([...states].sort()).toEqual(sync.entities.map((e) => e.state).sort());
    await expect(tauriPage.locator('[data-summary]')).toBeVisible();

    // A FAILING row must still show when the two sides last agreed. The writer
    // preserves `synced_at` through a failure precisely so this can be shown, and
    // dropping it would make broken-since-Tuesday read as never-synced.
    const failing = sync.entities.find((e) => e.state === 'error');
    if (failing) {
      const line = (await tauriPage.evaluate(
        `document.querySelector('[data-state="error"] [data-agreement]')?.textContent?.trim()`,
      )) as string | undefined;
      expect(line, 'a failing row reports its last agreement').toMatch(/failing/);
      expect(line, 'and the date it last agreed, not just that it is failing').toMatch(
        /\d{4}-\d{2}-\d{2}/,
      );
    }

    // And a skip must NOT read as a failure.
    const skipped = sync.entities.find((e) => e.state === 'skipped');
    if (skipped) {
      const line = (await tauriPage.evaluate(
        `document.querySelector('[data-state="skipped"] [data-agreement]')?.textContent?.trim()`,
      )) as string | undefined;
      expect(line).toMatch(/skipped/);
      expect(line, 'a deliberate skip is not a fault').not.toMatch(/failing/);
    }
  });

  test('shows every persona the daemon knows, with a standing line each', async ({
    tauriPage,
  }) => {
    const { personas } = await live<PersonasResponse>('/api/auth/personas');

    // A failed registry read must NOT render as "no identities" — that phrasing
    // invites connecting one, which is the wrong action against a registry that
    // is merely unreachable.
    await expect(tauriPage.locator('[data-persona-error]')).toHaveCount(0);

    if (personas.length === 0) {
      await expect(tauriPage.locator('[data-personas-empty]')).toBeVisible({ timeout: 15_000 });
      return;
    }

    const list = tauriPage.locator('[data-personas] [data-persona]');
    await expect(list).toHaveCount(personas.length, { timeout: 15_000 });

    // The standing line is the whole point of the screen: a credential must never
    // render as a blank row. Every persona gets a sentence, whatever its state.
    for (const p of personas) {
      const row = tauriPage.locator(`[data-persona="${p.label}"]`);
      await expect(row.locator('[data-standing]')).not.toHaveText('');
      // And a tone, so 30-minutes-left does not look like 7-hours-left.
      await expect(row).toHaveAttribute('data-tone', /idle|ok|warn|dead/);
    }
  });
});
