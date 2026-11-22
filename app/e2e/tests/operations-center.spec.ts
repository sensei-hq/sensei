/**
 * Operations center — the slice 9 Tauri matrix, live against the throwaway
 * e2e daemon and its `sensei_e2e` DB.
 *
 * 9.1 root-add initiates a scan; the center and the footer show it live,
 *     then settle (this is also 7.4's full cycle: root-add → queue →
 *     progress → done, in the center).
 * 9.2 a parent added over an inner root consolidates; coverage continues.
 * 9.3 file add/modify/delete is detected, queued, processed, shown.
 * 9.4 transcript/metric/sync waves run live end to end.
 * 9.5 a forced failure alerts with its reason, acks, retries, and clears
 *     (this is also 8.5).
 *
 * Every test cleans what it dirties (roots, consent, cadences, seeded rows):
 * the e2e database persists between runs, and a leftover root fails the next
 * spec that expects none.
 */

import { execFileSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, copyFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { test, expect, type TauriPageLike } from '../fixtures';
import {
  DAEMON_URL,
  daemonGet,
  daemonPost,
  navigateTo,
  removeRootsUnder,
  seedSetupComplete,
  waitFor,
} from '../helpers';
import { E2E_HOME } from '../isolated-home';
import { makeGitRepo, writeRepoFile, removeRepoFile } from '../lib/corpus';
import { captureSse } from '../lib/sse-capture';
import { psqlBin } from '../lib/psql';

const E2E_DB = 'sensei_e2e';

async function seedHealth(tauriPage: TauriPageLike): Promise<void> {
  await tauriPage.evaluate(`
    (function() {
      sessionStorage.setItem('sensei:health', 'ready');
      localStorage.removeItem('sensei:setup-complete');
    })()
  `);
}

function psqlE2e(sql: string): void {
  execFileSync(psqlBin(), ['-d', E2E_DB, '-v', 'ON_ERROR_STOP=1', '-c', sql], { stdio: 'pipe' });
}

interface QueueTotals {
  totals: { pending: number; blocked: number; running: number };
}

async function queueSettled(): Promise<boolean> {
  const q = await daemonGet<QueueTotals>('/api/tasks/queue?limit=1');
  return q.totals.pending === 0 && q.totals.blocked === 0 && q.totals.running === 0;
}

async function waitForSettle(timeoutMs = 240_000): Promise<void> {
  await waitFor(queueSettled, timeoutMs, 1000);
}

const isJobAdd = (e: { data: unknown }) => {
  const d = e.data as Record<string, unknown> | null;
  return !!d && d.entity === 'job' && d.action === 'add';
};

test.describe('Operations center matrix', () => {
  test.beforeEach(async ({ tauriPage }) => {
    test.setTimeout(420_000);
    await seedHealth(tauriPage);
    await seedSetupComplete(tauriPage);
  });

  test('9.1 root-add scans live in the center and the footer, then settles (7.4)', async ({
    tauriPage,
  }) => {
    const corpus = mkdtempSync(join(tmpdir(), 'sensei-e2e-ops91-'));
    const files: Record<string, string> = {};
    for (let i = 0; i < 30; i++) files[`src/mod${i}.py`] = `VALUE_${i} = ${i}\n`;
    makeGitRepo(join(corpus, 'repo'), files);
    const capture = captureSse(`${DAEMON_URL}/api/tasks/progress`);
    try {
      await daemonPost('/api/scan/roots', { path: corpus, excluded: [] });
      // The queue takes the work: a job add frame on the live stream.
      await capture.waitFor(isJobAdd, 60_000);

      await navigateTo(tauriPage, '/operations');
      await tauriPage.waitForSelector('[data-component="ops-queue"]', 30_000);

      // The scan of a small repo can settle before the page lands, so top the
      // queue up while asserting: each round adds files, then the center must
      // show live work within the round. Ten quiet rounds in a row fail.
      let seen = false;
      for (let round = 0; round < 10 && !seen; round++) {
        for (let i = 0; i < 5; i++) {
          writeRepoFile(join(corpus, 'repo'), `src/live${round}_${i}.py`, `X = ${round * i}\n`);
        }
        try {
          await waitFor(async () => {
            const count = await tauriPage.locator('[data-queue-pending], [data-now-running]').count();
            return count > 0;
          }, 20_000, 500);
          seen = true;
        } catch {
          // Next round tops the queue up again.
        }
      }
      expect(seen, 'the center shows live queue work while the scan runs').toBe(true);
      // The sidebar mini rides the same run — but the scan singleton polls
      // behind the center's live frames, so sustain the run while waiting
      // for it instead of asserting once after the drain.
      let miniSeen = false;
      for (let i = 0; i < 30 && !miniSeen; i++) {
        writeRepoFile(join(corpus, 'repo'), `src/mini${i}.py`, `M = ${i}\n`);
        await waitFor(async () => (await tauriPage.locator('[data-component="scan-mini"]').count()) > 0, 2000, 250).then(
          () => (miniSeen = true),
          () => {},
        );
      }
      expect(miniSeen, 'the footer mini shows while the scan runs').toBe(true);

      await waitForSettle();
      // Settled: the center idles honestly, and the mini stands down.
      // (No `text=` selectors: the socket harness does not support them —
      // poll textContent of a stable data hook instead.)
      await waitFor(async () => (await tauriPage.locator('[data-component="scan-mini"]').count()) === 0, 60_000, 1000);
      await waitFor(async () => {
        const text = await tauriPage.locator('[data-component="ops-queue"]').textContent();
        return (text ?? '').includes('Queue empty');
      }, 60_000, 1000);
    } finally {
      capture.close();
      // Settle first: pruning a root out from under its own in-flight scan
      // fails the scan's children (honestly — their tree is gone), so a
      // test that fails mid-scan must not poison the failed feed.
      await waitForSettle(60_000).catch(() => {});
      await removeRootsUnder(corpus);
    }
  });

  test('9.2 a parent added over an inner root consolidates; coverage continues', async ({
    tauriPage,
  }) => {
    const base = mkdtempSync(join(tmpdir(), 'sensei-e2e-ops92-'));
    const inner = join(base, 'inner');
    makeGitRepo(join(inner, 'repo'), { 'src/a.py': 'A = 1\n' });
    try {
      const added = await daemonPost<{ path: string; absorbed?: string[] }>('/api/scan/roots', {
        path: inner,
        excluded: [],
      });
      expect(added.path).toBe(inner);
      await waitForSettle();

      const workspaces = await daemonGet<Array<{ id: string }>>('/api/workspaces');
      expect(workspaces.length, 'the inner root indexes its repo').toBeGreaterThan(0);

      const parent = await daemonPost<{ path: string; absorbed?: Array<{ path: string }> }>(
        '/api/scan/roots',
        { path: base, excluded: [] },
      );
      expect(
        (parent.absorbed ?? []).map((a) => a.path),
        'the parent absorbs the inner root',
      ).toContain(inner);

      const roots = await daemonGet<Array<{ path: string }>>('/api/scan/roots');
      const ours = roots.filter((r) => r.path === base || r.path === inner).map((r) => r.path);
      expect(ours, 'only the parent remains').toEqual([base]);

      await waitForSettle();
      const after = await daemonGet<Array<{ id: string }>>('/api/workspaces');
      expect(after.length, 'coverage continues under the parent').toBeGreaterThan(0);

      await navigateTo(tauriPage, '/settings/roots');
      await tauriPage.waitForSelector(`button[aria-label="Remove ${base}"]`, 30_000);
    } finally {
      await waitForSettle(60_000).catch(() => {});
      await removeRootsUnder(base);
    }
  });

  test('9.3 file add, modify, and delete are detected, queued, and processed', async () => {
    const corpus = mkdtempSync(join(tmpdir(), 'sensei-e2e-ops93-'));
    const repo = join(corpus, 'repo');
    makeGitRepo(repo, { 'src/keep.py': 'KEEP = 1\n' });
    const capture = captureSse(`${DAEMON_URL}/api/tasks/progress`);
    try {
      await daemonPost('/api/scan/roots', { path: corpus, excluded: [] });
      await waitForSettle();

      const jobFor = (pathPart: string) => (e: { data: unknown }) => {
        const d = e.data as Record<string, unknown> | null;
        if (!d || d.entity !== 'job' || d.action !== 'add') return false;
        const data = d.data as Record<string, unknown>;
        return typeof data.path === 'string' && data.path.includes(pathPart);
      };

      writeRepoFile(repo, 'src/added.py', 'ADDED = 1\n');
      await capture.waitFor(jobFor('added.py'), 90_000);

      writeRepoFile(repo, 'src/keep.py', 'KEEP = 2\n');
      await capture.waitFor(jobFor('keep.py'), 90_000);

      removeRepoFile(repo, 'src/keep.py');
      await capture.waitFor(jobFor('keep.py'), 90_000);

      await waitForSettle();
      // Scoped to this corpus: the feed is global, and another spec's debris
      // must never fail this one.
      const failed = await daemonGet<{ tasks: Array<{ path: string }> }>('/api/tasks/failed?days=1&limit=100');
      expect(
        failed.tasks.filter((t) => t.path.startsWith(corpus)),
        'the watcher-driven work processes cleanly',
      ).toEqual([]);
    } finally {
      capture.close();
      await waitForSettle(60_000).catch(() => {});
      await removeRootsUnder(corpus);
    }
  });

  test('9.4 transcript, metric, and sync waves run live end to end', async ({ tauriPage }) => {
    // The metric pass needs an indexed workspace of its own — self-sufficient,
    // never borrowed from an earlier test's scan.
    const corpus = mkdtempSync(join(tmpdir(), 'sensei-e2e-ops94-'));
    makeGitRepo(join(corpus, 'repo'), { 'src/m.py': 'M = 1\n' });
    await daemonPost('/api/scan/roots', { path: corpus, excluded: [] });
    await waitForSettle();
    // Transcript fixtures are NEVER hand-written (the leak carve-out): copy
    // the anonymised claude fixture into the isolated home's projects dir.
    const projects = join(E2E_HOME, '.claude', 'projects', 'e2e-ops94');
    mkdirSync(projects, { recursive: true });
    const here = dirname(fileURLToPath(import.meta.url));
    const repoRoot = join(here, '..', '..', '..');
    copyFileSync(
      join(repoRoot, 'crates/senseid/tests/fixtures/transcripts/claude/session-with-tools.jsonl'),
      join(projects, 'e2e-session.jsonl'),
    );
    const capture = captureSse(`${DAEMON_URL}/api/tasks/progress`);
    const patched: Array<{ name: string; interval_secs: number }> = [];
    try {
      // Consent the source, then backfill: totals up front, a terminal with
      // settled counts at the end.
      const consent = await fetch(`${DAEMON_URL}/api/transcripts/consent/claude_code`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ consented: true }),
      });
      expect(consent.ok, 'consent grants').toBe(true);
      const backfill = (await (
        await fetch(`${DAEMON_URL}/api/transcripts/backfill`, { method: 'POST' })
      ).json()) as { taskId?: number };
      expect(typeof backfill.taskId, 'backfill enqueues a wave').toBe('number');
      const waveKey = String(backfill.taskId);
      await capture.waitFor((e) => {
        const d = e.data as Record<string, unknown> | null;
        return !!d && d.entity === 'transcript' && d.action === 'add' && d.key === waveKey;
      }, 120_000);
      const terminal = await capture.waitFor((e) => {
        const d = e.data as Record<string, unknown> | null;
        return !!d && d.entity === 'transcript' && d.key === waveKey && d.done === true;
      }, 180_000);
      const tdata = (terminal.data as Record<string, unknown>).data as Record<string, unknown>;
      expect(tdata.settled, 'the transcript wave settles its sessions').toBeGreaterThan(0);

      // Metric + sync passes on shortened cadences, awaited together.
      const sched = await daemonGet<{ tasks: Array<{ name: string; interval_secs: number }> }>(
        '/api/tasks/scheduled',
      );
      for (const name of ['metrics', 'dojo_sync']) {
        const row = sched.tasks.find((t) => t.name === name);
        if (row) patched.push({ name, interval_secs: row.interval_secs });
        await fetch(`${DAEMON_URL}/api/tasks/scheduled/${name}`, {
          method: 'PATCH',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ interval_secs: 30 }),
        });
      }
      const [metricDone, syncDone] = await Promise.all([
        capture.waitFor((e) => {
          const d = e.data as Record<string, unknown> | null;
          return !!d && d.entity === 'metric' && d.done === true;
        }, 240_000),
        capture.waitFor((e) => {
          const d = e.data as Record<string, unknown> | null;
          return !!d && d.entity === 'sync' && d.done === true;
        }, 240_000),
      ]);
      expect(metricDone, 'the metric wave ends').toBeTruthy();
      expect(syncDone, 'the sync pass ends (empty: no signed-in personas)').toBeTruthy();

      // The center shows the settled waves after the fact.
      await navigateTo(tauriPage, '/operations');
      await tauriPage.waitForSelector('[data-component="ops-pipelines"]', 30_000);
      await waitFor(async () => (await tauriPage.locator('[data-pipeline-wave]').count()) > 0, 60_000, 1000);
    } finally {
      capture.close();
      for (const { name, interval_secs } of patched) {
        await fetch(`${DAEMON_URL}/api/tasks/scheduled/${name}`, {
          method: 'PATCH',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ interval_secs }),
        }).catch(() => {});
      }
      await fetch(`${DAEMON_URL}/api/transcripts/consent/claude_code`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ consented: false }),
      }).catch(() => {});
      rmSync(projects, { recursive: true, force: true });
      await removeRootsUnder(corpus);
    }
  });

  test('9.5 a forced failure alerts with its reason, acks, retries, and clears (8.5)', async ({
    tauriPage,
  }) => {
    // Ack dispositions persist: a row left by a killed run would render this
    // alert pre-acked (no button) and fail the ack step. Clear it on the way
    // in as well as the way out.
    psqlE2e(`DELETE FROM activity.alert_acks WHERE class = 'task' AND alert_key = '900001'`);
    psqlE2e(
      `INSERT INTO activity.task_executions(task_id, task_kind, folder_path, path, status, error_message) VALUES
       (900001, 'process_file', '/r', '/r/e2e-forced.ts', 'failed', 'e2e forced failure')`,
    );
    try {
      await navigateTo(tauriPage, '/operations');
      await tauriPage.waitForSelector('[data-component="ops-failures"]', 30_000);
      // The feed refreshes on attach: the forced failure lands with its reason.
      await waitFor(async () => {
        const text = await tauriPage.locator('[data-component="ops-failures"]').textContent();
        return (text ?? '').includes('e2e forced failure');
      }, 60_000, 1000);
      // And the alert stands beside it.
      await waitFor(async () => (await tauriPage.locator('[data-alert-ack="task:900001"]').count()) > 0, 60_000, 1000);

      // Ack silences without deleting.
      await tauriPage.locator('[data-alert-ack="task:900001"]').click();
      await waitFor(async () => {
        const text = await tauriPage.locator('[data-component="ops-alerts"]').textContent();
        return (text ?? '').includes('Acked');
      }, 30_000, 500);

      // Retry re-queues through the daemon.
      await tauriPage.locator('[data-failure-retry="900001"]').click();
      await waitFor(async () => {
        const text = await tauriPage.locator('[data-component="ops-failures"]').textContent();
        return (text ?? '').includes('re-queued as #');
      }, 30_000, 500);

      // Clearing the condition clears the alert (success-clear). Remount via
      // navigation so the derivation re-runs at once instead of on the next
      // 30s poll.
      psqlE2e('DELETE FROM activity.task_executions WHERE task_id = 900001');
      await tauriPage.locator('[data-failures-refresh]').click();
      await navigateTo(tauriPage, '/operations');
      await tauriPage.waitForSelector('[data-component="ops-alerts"]', 30_000);
      await waitFor(async () => (await tauriPage.locator('[data-alert-ack="task:900001"]').count()) === 0, 30_000, 500);
    } finally {
      psqlE2e('DELETE FROM activity.task_executions WHERE task_id = 900001');
      psqlE2e(`DELETE FROM activity.alert_acks WHERE class = 'task' AND alert_key = '900001'`);
    }
  });
});
