/**
 * Scan tracker: sample the operations center + queue API on a cadence while a
 * large scan runs, appending one JSON row per round to
 * /tmp/sensei-scan-track.jsonl. Read-only: navigates to /operations once,
 * then polls DOM state and the daemon API — no clicks, no mutations.
 *
 * Run: `bun run test:inspect -- scan-tracker` (needs the real daemon up).
 * Knobs: TRACK_ROUNDS (default 30), TRACK_INTERVAL_MS (default 60000).
 */
import { appendFileSync } from 'node:fs';
import { test, type TauriPageLike } from '../fixtures';
import { DAEMON_URL, PORT, navigateTo, waitFor } from '../helpers';

const OUT = '/tmp/sensei-scan-track.jsonl';
const ROUNDS = Number(process.env.TRACK_ROUNDS ?? 30);
const INTERVAL_MS = Number(process.env.TRACK_INTERVAL_MS ?? 60_000);

interface RootsRow {
  path: string;
  status: string;
  repos_found: number;
  scanned: boolean;
}

async function api<T>(path: string): Promise<T | null> {
  try {
    const r = await fetch(`${DAEMON_URL}${path}`);
    if (!r.ok) return null;
    return (await r.json()) as T;
  } catch {
    return null;
  }
}

async function domCount(page: TauriPageLike, selector: string): Promise<number | null> {
  try {
    return await page.locator(selector).count();
  } catch {
    return null;
  }
}

test('scan tracker samples the center until the rounds run out', async ({ tauriPage }) => {
  test.setTimeout(ROUNDS * INTERVAL_MS + 600_000);
  await navigateTo(tauriPage, '/operations');
  await tauriPage.waitForSelector('[data-component="ops-queue"]', 60_000);

  for (let round = 0; round < ROUNDS; round++) {
    const t0 = Date.now();
    const queue = await api<{ totals: { pending: number; blocked: number; running: number } }>(
      '/api/tasks/queue?limit=1',
    );
    const roots = await api<RootsRow[]>('/api/scan/roots');
    const row = {
      t: new Date().toISOString(),
      round,
      port: PORT,
      queue: queue?.totals ?? null,
      roots: roots?.map((r) => ({ path: r.path, status: r.status, repos: r.repos_found, scanned: r.scanned })) ?? null,
      dom: {
        queuePending: await domCount(tauriPage, '[data-queue-pending]'),
        nowRunning: await domCount(tauriPage, '[data-now-running]'),
        pipelineWaves: await domCount(tauriPage, '[data-pipeline-wave]'),
        pipelineHistory: await domCount(tauriPage, '[data-pipeline-history]'),
        failureRows: await domCount(tauriPage, '[data-failure-row]'),
        alertRows: await domCount(tauriPage, '[data-alert-row]'),
      },
    };
    appendFileSync(OUT, `${JSON.stringify(row)}\n`);
    console.log(
      `[track] r${round} q=${queue ? `${queue.totals.pending}p/${queue.totals.blocked}b/${queue.totals.running}r` : 'n/a'} ` +
        `dom(fail=${row.dom.failureRows},alerts=${row.dom.alertRows})`,
    );
    // Re-assert the page is alive each round; a dead webview must fail loud,
    // not log nulls for an hour.
    await waitFor(async () => (await domCount(tauriPage, '[data-component="ops-queue"]')) === 1, 30_000, 1000);
    const wait = INTERVAL_MS - (Date.now() - t0);
    if (round < ROUNDS - 1 && wait > 0) await new Promise((r) => setTimeout(r, wait));
  }
});
