/**
 * Observatory daily-view coverage — the post-T3 mockup-gap slice:
 *   • Today: FtrStrip bar strip + "first-try" / "N× rework" corrections column.
 *   • Projects list: renders project cards (filtered by status).
 *   • Insights: Now / Soon / Settled triage columns (or honest empty).
 *   • Upgrades / Impact: real buckets (or honest empty state).
 *
 * Gated routes are driven with navigateToScreen (retries through the health
 * gate); selectors are the app's stable data-* hooks.
 */

import { test, expect } from '../fixtures';
import { navigateToScreen, DAEMON_URL } from '../helpers';

/** Fetch and JSON-parse defensively — the E2E daemon returns 0-byte bodies for
 *  some empty-state endpoints. */
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
    const sessions = await safeJson<{ sessions: Array<unknown> }>(
      `${DAEMON_URL}/api/sessions`, { sessions: [] },
    );
    if (sessions.sessions.length === 0) {
      test.skip(true, 'no sessions captured yet on this daemon');
      return;
    }
    await navigateToScreen(tauriPage, '/', '[data-component="observatory-main"]');
    await tauriPage.waitForSelector('[data-session-row]', 15_000);
    const text = await tauriPage.evaluate(`
      Array.from(document.querySelectorAll('[data-session-row] [data-corrections]'))
        .map(el => el.textContent?.trim() ?? '')
        .join('|')
    `) as string;
    expect(text).toMatch(/first-try|× rework|—/);
  });

  test('mature-mode FTR header renders the FtrStrip bar strip', async ({ tauriPage }) => {
    const ftr = await safeJson<{ ftr_daily: Array<{ ftr_rate: number }> }>(
      `${DAEMON_URL}/api/observatory/ftr-daily`, { ftr_daily: [] },
    );
    if (ftr.ftr_daily.length < 2) {
      test.skip(true, 'FtrStrip is only rendered in mature mode (≥ 2 daily buckets)');
      return;
    }
    await navigateToScreen(tauriPage, '/', '[data-component="observatory-main"]');
    await tauriPage.waitForSelector('[data-component="ftr-strip"]', 15_000);
    const todayBar = await tauriPage.locator('[data-testid="ftr-bar-today"]').count();
    expect(todayBar).toBeGreaterThan(0);
  });
});

test.describe('Observatory — Projects list', () => {
  test('renders project cards (filtered by status), or an honest empty state', async ({ tauriPage }) => {
    const projects = await safeJson<Array<unknown>>(`${DAEMON_URL}/api/projects`, []);
    if (projects.length === 0) {
      test.skip(true, 'no projects registered on this daemon');
      return;
    }
    await navigateToScreen(tauriPage, '/projects', '[data-component="projects-page"]');
    // Segmentation is now a status FILTER (All/Active/Dormant/Archived); the
    // list renders project cards/rows. With ≥1 project registered, at least one
    // card is visible (default filter is All).
    const cards = await tauriPage
      .locator('[data-project-card], [data-project-row]')
      .count();
    expect(cards).toBeGreaterThan(0);
  });
});

test.describe('Observatory — Insights triage', () => {
  test('renders Now / Soon / Settled columns (or an honest empty state)', async ({ tauriPage }) => {
    await navigateToScreen(tauriPage, '/insights', '[data-component="observatory-main"]');
    await tauriPage.waitForSelector('[data-triage-grid], [data-empty]', 15_000);
    const columns = await tauriPage.evaluate(`
      Array.from(document.querySelectorAll('[data-triage-column]')).map(el => el.getAttribute('data-triage-column'))
    `) as string[];
    if (columns.length === 0) return; // Legit empty state.
    for (const c of ['now', 'soon', 'settled']) expect(columns).toContain(c);
  });
});

test.describe('Observatory — Upgrades', () => {
  test('renders installable-recommendation buckets or an honest empty state', async ({ tauriPage }) => {
    await navigateToScreen(tauriPage, '/upgrades', '[data-component="observatory-main"]');
    await tauriPage.waitForSelector('[data-upgrades-total], [data-empty]', 15_000);
    const total = await tauriPage.evaluate(`
      document.querySelector('[data-upgrades-total]')?.getAttribute('data-upgrades-total')
    `) as string | null;
    if (total == null || total === '0') return; // Honest empty state.
    const buckets = await tauriPage.evaluate(`
      Array.from(document.querySelectorAll('[data-upgrade-bucket]')).map(el => el.getAttribute('data-upgrade-bucket'))
    `) as string[];
    expect(buckets.length).toBeGreaterThan(0);
    for (const b of buckets) expect(['skill', 'agent', 'rule', 'lint', 'other']).toContain(b);
  });
});

test.describe('Observatory — Impact', () => {
  test('renders verdict buckets or an honest empty state', async ({ tauriPage }) => {
    await navigateToScreen(tauriPage, '/impact', '[data-component="observatory-main"]');
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
