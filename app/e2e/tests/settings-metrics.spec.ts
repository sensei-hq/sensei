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
import { navigateTo, DAEMON_URL } from '../helpers';

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

async function seedHealth(tauriPage: any): Promise<void> {
  await tauriPage.evaluate(`
    (function() {
      sessionStorage.setItem('sensei:health', 'ready');
      localStorage.removeItem('sensei:setup-complete');
    })()
  `);
}

test.describe('Settings · Metrics', () => {
  test.beforeEach(async ({ tauriPage }) => {
    await seedHealth(tauriPage);
    await navigateTo(tauriPage, '/settings/metrics');
  });

  test('lists the real estate, each repository exactly once', async ({ tauriPage }) => {
    const live = (await fetch(`${DAEMON_URL}/api/metrics/status/summary`).then((r) =>
      r.json(),
    )) as Summary;
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
    const live = (await fetch(`${DAEMON_URL}/api/metrics/status/summary`).then((r) =>
      r.json(),
    )) as Summary;
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
    const live = (await fetch(`${DAEMON_URL}/api/metrics/status/summary`).then((r) =>
      r.json(),
    )) as Summary;
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
