/**
 * Observatory daily-view coverage — the post-T3 mockup-gap slice:
 *   • Today: FtrStrip bar strip + "first-try" / "N× rework" corrections
 *     column on Recent Sessions.
 *   • Projects list: segmented Active / Recent / Archived cards.
 *   • Insights: Now / Soon / Settled triage columns.
 *   • Upgrades: real installable-recommendation buckets (or honest empty state).
 *   • Observatory Impact: measured verdict buckets (or honest empty state).
 *
 * Each test walks the DOM the real user hits — the tone matches how the
 * observatory renders on an early-mode install (mode="early") vs a
 * mature install. On a fresh sensei_e2e DB the observatory is early —
 * mockup-gap tests still assert the structural pieces we control.
 */

import { test, expect } from '../fixtures';
import { navigateTo, DAEMON_URL } from '../helpers';

/** Fetch and JSON-parse defensively — the E2E daemon returns 0-byte
 *  bodies for some empty-state endpoints. Same helper the T3 flows spec
 *  uses. */
async function safeJson<T>(url: string, fallback: T): Promise<T> {
  try {
    const res = await fetch(url);
    if (!res.ok) return fallback;
    const text = await res.text();
    if (!text) return fallback;
    return JSON.parse(text) as T;
  } catch {
    return fallback;
  }
}

test.describe('Observatory — Today', () => {
  test('Recent Sessions column formats corrections as first-try / N× rework', async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/');
    // The wide layout may be either the mature hero or the early listening
    // hero; both include RecentSessions when at least one session lands.
    // Poll the endpoint so the test isn't order-dependent with capture.
    const sessions = await safeJson<{ sessions: Array<unknown> }>(
      `${DAEMON_URL}/api/sessions`, { sessions: [] },
    );
    if (sessions.sessions.length === 0) {
      test.skip(true, 'no sessions captured yet on this daemon');
      return;
    }
    // At least one session row should render the label vocabulary.
    await tauriPage.waitForSelector('[data-session-row]', 15_000);
    const text = await tauriPage.evaluate(`
      Array.from(document.querySelectorAll('[data-session-row] [data-corrections]'))
        .map(el => el.textContent?.trim() ?? '')
        .join('|')
    `) as string;
    // Every row should carry EITHER "first-try", "N× rework", or "—".
    expect(text).toMatch(/first-try|× rework|—/);
  });

  test('mature-mode FTR header renders the FtrStrip bar strip', async ({ tauriPage }) => {
    // Only assert the strip when the daemon has FTR data. Otherwise the
    // early hero is expected instead — that's a legitimate empty state.
    const ftr = await safeJson<{ ftr_daily: Array<{ ftr_rate: number }> }>(
      `${DAEMON_URL}/api/observatory/ftr-daily`, { ftr_daily: [] },
    );
    if (ftr.ftr_daily.length < 2) {
      test.skip(true, 'FtrStrip is only rendered in mature mode (≥ 2 daily buckets)');
      return;
    }
    await navigateTo(tauriPage, '/');
    await tauriPage.waitForSelector('[data-component="ftr-strip"]', 15_000);
    const todayBar = await tauriPage.locator('[data-testid="ftr-bar-today"]').count();
    expect(todayBar).toBeGreaterThan(0);
  });
});

test.describe('Observatory — Projects list', () => {
  test('renders segmented Active / Recent / Archived sections', async ({ tauriPage }) => {
    const projects = await safeJson<Array<unknown>>(`${DAEMON_URL}/api/projects`, []);
    if (projects.length === 0) {
      test.skip(true, 'no projects registered on this daemon');
      return;
    }
    await navigateTo(tauriPage, '/projects');
    // At LEAST one bucket should be visible (typically Recent on a fresh
    // e2e DB where no sessions have landed in the last 7 days).
    await tauriPage.waitForSelector('[data-bucket]', 15_000);
    const buckets = await tauriPage.evaluate(`
      Array.from(document.querySelectorAll('[data-bucket]')).map(el => el.getAttribute('data-bucket'))
    `) as string[];
    // All present buckets must be one of the three legal values.
    for (const b of buckets) expect(['active', 'recent', 'archived']).toContain(b);
    expect(buckets.length).toBeGreaterThan(0);
  });
});

test.describe('Observatory — Insights triage', () => {
  test('renders Now / Soon / Settled columns (or an honest empty state)', async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/insights');
    // Wait for either the triage grid OR the empty-state kanji before
    // asserting — one of the two ALWAYS renders. `data-empty` is on the
    // shared EmptyState component when triage.total === 0.
    await tauriPage.waitForSelector('[data-triage-grid], [data-empty]', 15_000);
    const columns = await tauriPage.evaluate(`
      Array.from(document.querySelectorAll('[data-triage-column]')).map(el => el.getAttribute('data-triage-column'))
    `) as string[];
    if (columns.length === 0) {
      // Legit empty state — nothing more to check.
      return;
    }
    // All three columns must be present in mature mode.
    for (const c of ['now', 'soon', 'settled']) expect(columns).toContain(c);
  });
});

test.describe('Observatory — Upgrades', () => {
  test('renders installable-recommendation buckets or an honest empty state', async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/upgrades');
    // Either the buckets container carries the total, or EmptyState renders.
    await tauriPage.waitForSelector('[data-upgrades-total], [data-empty]', 15_000);
    const total = await tauriPage.evaluate(`
      document.querySelector('[data-upgrades-total]')?.getAttribute('data-upgrades-total')
    `) as string | null;
    if (total == null || total === '0') return; // Honest empty state.
    // At least one bucket in the ordered set must be present.
    const buckets = await tauriPage.evaluate(`
      Array.from(document.querySelectorAll('[data-upgrade-bucket]')).map(el => el.getAttribute('data-upgrade-bucket'))
    `) as string[];
    expect(buckets.length).toBeGreaterThan(0);
    for (const b of buckets) expect(['skill', 'agent', 'rule', 'lint', 'other']).toContain(b);
  });
});

test.describe('Observatory — Impact', () => {
  test('renders verdict buckets or an honest empty state', async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/impact');
    await tauriPage.waitForSelector('[data-impact-total], [data-empty]', 15_000);
    const total = await tauriPage.evaluate(`
      document.querySelector('[data-impact-total]')?.getAttribute('data-impact-total')
    `) as string | null;
    if (total == null || total === '0') return; // Honest empty state.
    const buckets = await tauriPage.evaluate(`
      Array.from(document.querySelectorAll('[data-impact-bucket]')).map(el => el.getAttribute('data-impact-bucket'))
    `) as string[];
    expect(buckets.length).toBeGreaterThan(0);
    for (const b of buckets) expect(['positive', 'negative', 'neutral', 'pending']).toContain(b);
  });
});
