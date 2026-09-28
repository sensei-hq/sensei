/**
 * Settings · Metrics — the screen that answers "why is there no row for today?".
 *
 * Unit tests cover the ranking, the labels and the toggle controller against a
 * mock. What only an e2e can prove is that the screen is wired to the DAEMON and
 * not to a fixture — so every assertion here is cross-checked against a live read
 * of the same endpoint, rather than against a hardcoded expectation.
 *
 * The two properties worth an e2e:
 *
 *  1. The rail lists the real estate, and a repository whose metrics carry
 *     SEVERAL reason codes appears ONCE. That is the defect a run-length grouping
 *     shipped with, and live data has repositories at three codes — so a split
 *     would show the same repository two or three times with partial counts.
 *  2. Every rendered reason resolves to a sentence from `sensei.reason_codes`.
 *     A bare slug on screen means the vocabulary did not travel with the read,
 *     which is the exact failure the registry exists to prevent.
 */

import { test, expect } from '../fixtures';
import { navigateToScreen, DAEMON_URL } from '../helpers';

interface SummaryRow {
  repository_id: string;
  repo_key: string | null;
  name: string;
  by_reason: Record<string, number>;
  total: number;
}
interface Summary {
  count: number;
  repositories: SummaryRow[];
  reasons: Record<string, { summary: string; kind: string }>;
}

/**
 * The live summary, with the status checked before the body is parsed.
 *
 * `r.json()` on a failed response throws "Unexpected end of JSON input" — the
 * daemon's error responses carry a bare status and no body. That message sent an
 * investigation looking for a serialisation bug when the real cause was a 500
 * from a database missing the `sensei.metric_status` view. Assert the status, and
 * the failure names itself.
 */
async function liveSummary(): Promise<Summary> {
  const r = await fetch(`${DAEMON_URL}/api/metrics/status/summary`);
  expect(
    r.ok,
    `GET /api/metrics/status/summary returned ${r.status} — the e2e daemon's ` +
      `database is missing the metric_status view or the metric_computation ` +
      `reason codes`,
  ).toBe(true);
  return (await r.json()) as Summary;
}

/**
 * Past both gates the observatory sits behind: the health check, and setup.
 *
 * Setup completion is DAEMON state (`PUT /api/config`), not just a localStorage
 * flag — writing only the flag leaves the app on the setup wizard, and the
 * assertions then run against the welcome page instead of the screen under test.
 * Several older specs `removeItem` it and still reach their screen; that works
 * only on a database where setup already happened, which a fresh e2e DB is not.
 * Mirrors `atlas.spec.ts`, which survives a fresh DB.
 */
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

test.describe('Settings · Metrics', () => {
  test.beforeEach(async ({ tauriPage }) => {
    // Cold health-bootstrap plus the setup reconcile for the first gated screen
    // can take ~50s on a loaded box.
    test.setTimeout(180_000);
    await seedSetupComplete(tauriPage);
    // `navigateToScreen`, not `navigateTo`: the observatory shell is gated on
    // health + setup, and `wizardState.setupComplete` only reconciles from the
    // daemon config on a health TRANSITION — so a single navigation right after
    // seeding lands on /setup and no amount of waiting on the target selector
    // helps, because we are on another route. This re-navigates until it mounts.
    await navigateToScreen(tauriPage, '/settings/metrics', '[data-screen="settings-metrics"]');
  });

  /**
   * The unconditional gate. Every other test here needs repositories, and a fresh
   * e2e database has none — so they skip, and a suite of skips proves nothing.
   *
   * This one runs on any install and asserts the property that actually breaks:
   * the screen distinguishes "no repositories" from "the read failed". Honest-empty
   * is only correct when the data genuinely is empty, and an error state rendered
   * over a working-but-empty install (or a blank rail hiding a 500) are the two
   * ways this goes wrong.
   */
  test('renders against the daemon, and empty is not an error', async ({ tauriPage }) => {
    const live = await liveSummary();

    // `data-screen-state`, hyphenated — the attribute ScreenState actually emits.
    // An earlier version asserted `[data-screenstate="error"]`, which matches
    // nothing and so passed no matter what the screen showed: a check that cannot
    // fail is worse than no check, because it reads as coverage.
    await expect(tauriPage.locator('[data-screen-state="error"]')).toHaveCount(0);

    // Named so a failure below reports what the screen SHOWED, rather than only
    // that some selector was missing.
    const shown = async () =>
      (await tauriPage.locator('body').innerText()).slice(0, 400).replace(/\n+/g, ' | ');

    if (live.count === 0) {
      // Empty says so in words. The rail is a bordered box with no intrinsic
      // height, so rendering it empty beside "choose a repository" would show an
      // invisible list and an instruction that cannot be followed — which reads
      // as broken, not as empty. This assertion is what caught that.
      await expect(
        tauriPage.locator('[data-empty]'),
        `expected the honest-empty message; screen showed: ${await shown()}`,
      ).toBeVisible({ timeout: 15_000 });
      await expect(tauriPage.locator('[data-repo-rail]')).toHaveCount(0);
      test.info().annotations.push({
        type: 'coverage-gap',
        description:
          'Fresh e2e database has 0 repositories, so the rail/reason/toggle tests ' +
          'below skip. They bite on a populated install; seeding a metric fixture ' +
          'for e2e needs a sensei.repositories row, which no daemon endpoint ' +
          'creates (POST /api/repos makes a PROJECT). Tracked with the metrics backlog.',
      });
      return;
    }

    const rail = tauriPage.locator('[data-repo-rail]');
    await expect(rail).toBeVisible({ timeout: 15_000 });
    await expect(rail.locator('[data-repo]')).toHaveCount(live.count, { timeout: 15_000 });
    // The prompt is shown before anything is chosen — not a blank pane that reads
    // as "loaded, and there is nothing".
    await expect(tauriPage.getByText('Choose a repository')).toBeVisible({
      timeout: 15_000,
    });
  });

  test('lists the real estate, each repository exactly once', async ({ tauriPage }) => {
    const live = await liveSummary();
    if (live.count === 0) {
      test.skip(true, 'No repositories registered — nothing to render');
      return;
    }

    const rail = tauriPage.locator('[data-repo-rail]');
    await expect(rail).toBeVisible({ timeout: 15_000 });

    // Count from the DAEMON, not a literal: the fixture install's repository
    // count is not this test's business, only that the screen shows all of them.
    await expect(rail.locator('[data-repo]')).toHaveCount(live.count, { timeout: 15_000 });

    // The merge property, exercised on whichever repository actually has more
    // than one reason code. Skipped rather than faked if none does — asserting it
    // against a single-code repository would pass whatever the grouping does.
    const multi = live.repositories.find((r) => Object.keys(r.by_reason).length > 1);
    if (!multi) {
      test.info().annotations.push({
        type: 'note',
        description: 'No repository has 2+ reason codes; merge property not exercised',
      });
      return;
    }
    await expect(rail.locator(`[data-repo="${multi.repository_id}"]`)).toHaveCount(1);
  });

  test('every reason on screen is a sentence, never a bare code', async ({ tauriPage }) => {
    const live = await liveSummary();
    if (live.count === 0) {
      test.skip(true, 'No repositories registered');
      return;
    }

    const first = live.repositories[0];
    const rail = tauriPage.locator('[data-repo-rail]');
    await expect(rail).toBeVisible({ timeout: 15_000 });
    await rail.locator(`[data-repo="${first.repository_id}"]`).click();

    const rows = tauriPage.locator('[data-metric-rows] [data-metric]');
    await expect(rows.first()).toBeVisible({ timeout: 15_000 });
    await expect(rows).toHaveCount(first.total, { timeout: 15_000 });

    // Each row's rendered line must equal the registry's summary for its code —
    // so a slug, a blank, or an invented fallback all fail.
    const codes = new Set<string>();
    for (const row of await rows.all()) {
      const code = await row.getAttribute('data-reason');
      expect(code).toBeTruthy();
      codes.add(code!);
      const expected = live.reasons[code!]?.summary;
      expect(expected, `reason "${code}" is in the served vocabulary`).toBeTruthy();
      await expect(row.locator('[data-reason-summary]')).toHaveText(expected!);
    }

    // Sanity: the codes on screen are the ones the summary counted for this
    // repository. A mismatch means the two reads disagree about the same rows.
    expect([...codes].sort()).toEqual(Object.keys(first.by_reason).sort());
  });

  test('a repository with no remote says why it cannot be configured', async ({
    tauriPage,
  }) => {
    const live = await liveSummary();
    const keyless = live.repositories.find((r) => r.repo_key === null);
    if (!keyless) {
      test.skip(true, 'No local-only repository on this install');
      return;
    }

    const rail = tauriPage.locator('[data-repo-rail]');
    await expect(rail).toBeVisible({ timeout: 15_000 });
    await rail.locator(`[data-repo="${keyless.repository_id}"]`).click();

    // The reason is stated, not implied by a greyed-out control: activation is
    // decided per repo_key, and a repository with no remote has none.
    await expect(tauriPage.locator('[data-not-configurable]')).toBeVisible({
      timeout: 15_000,
    });
  });
});
