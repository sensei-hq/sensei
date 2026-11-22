/**
 * Crash recovery — the slice 4 kill-restart matrix, live against the
 * throwaway e2e daemon and its `sensei_e2e` DB. Also proves 3.2–3.4, 3.6–3.7.
 *
 * 4.1 index crash resumes without redo (3.6): kill mid-scan, restart, every
 *     file parsed exactly once, folder reaches indexed.
 * 4.2 wave crash abandons and re-drives (3.2): kill mid-backfill, the dead
 *     wave reads abandoned, a fresh backfill covers the missed sessions.
 * 4.3 orphaned tasks return with lineage; the drill-down shows the chain.
 * 4.4 acks survive reload; recurrence escalates across restart.
 * 4.5 metric boot backfill terminals after restart (3.3); sync boot pass
 *     terminals with no error (3.4); stale running_since clears (3.7).
 *
 * Kills are `kill -9` (a crash), never the helper's graceful `/stop`.
 * Every test cleans what it dirties; the transcript test restores an
 * unflagged daemon before it leaves (see below).
 */

import { execFileSync, execSync } from 'node:child_process';
import { existsSync, mkdtempSync, mkdirSync, copyFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { test, expect, type TauriPageLike } from '../fixtures';
import {
  DAEMON_URL,
  PORT,
  daemonGet,
  daemonPost,
  installErrorTrap,
  navigateToScreen,
  readErrors,
  removeRootsUnder,
  seedSetupComplete,
  waitFor,
} from '../helpers';
import { E2E_HOME, isolatedHomeEnvVars } from '../isolated-home';
import { makeGitRepo, writeRepoFile } from '../lib/corpus';
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

/** Cleanup that must not mask the real failure: a dead daemon at teardown
 *  (the test already failed at the kill) throws ECONNREFUSED out of the
 *  finally and the report blames cleanup instead of the kill. */
async function bestEffort(fn: () => Promise<unknown>): Promise<void> {
  try {
    await fn();
  } catch (e) {
    console.log(`[crash-cleanup] best-effort cleanup failed: ${(e as Error)?.message ?? e}`);
  }
}

function psqlE2e(sql: string): string {
  return execFileSync(psqlBin(), ['-d', E2E_DB, '-Atqv', 'ON_ERROR_STOP=1', '-c', sql], {
    stdio: 'pipe',
    encoding: 'utf8',
  }) as string;
}

/** True crash: SIGKILL the daemon's listener, then wait the port out. LISTEN
 *  only: a bare port match would also kill this worker (its SSE capture
 *  holds the port open) and the app under test — a suicide that reads as a
 *  runner crash with no test output. */
async function killDaemon(): Promise<void> {
  const holders = (): string[] => {
    try {
      return execSync(`lsof -ti :${PORT} -sTCP:LISTEN`, { stdio: 'pipe', encoding: 'utf8' })
        .trim()
        .split('\n')
        .filter(Boolean);
    } catch {
      return [];
    }
  };
  for (const pid of holders()) {
    // Never kill ourselves: the worker must not hold LISTEN, but if it
    // ever does, skipping our own PID turns suicide into a loud failure.
    if (Number(pid) === process.pid) throw new Error('killDaemon matched the test worker');
    try {
      process.kill(Number(pid), 'SIGKILL');
    } catch { /* already gone */ }
  }
  await waitFor(async () => holders().length === 0, 15_000, 500);
}

/**
 * Crash the daemon and bring it back up. The app resolves (and spawns) the
 * daemon exactly once, at boot, via DaemonStartResolver stage 2 — the boot
 * gate reads a cached flag and never re-probes, so a mid-run kill has no
 * app-owned reboot and waiting for one always times out. The spec therefore
 * owns the respawn, mirroring stage 2 exactly: `which senseid` (the same
 * binary the app booted), `start --instance e2e`, the app-boot env.
 */
async function spawnE2eDaemon(): Promise<void> {
  const bin = execSync('which senseid', { stdio: 'pipe', encoding: 'utf8' }).trim();
  if (!bin) throw new Error('spawnE2eDaemon: `which senseid` resolved nothing');
  const here = dirname(fileURLToPath(import.meta.url));
  const ddlDir = join(here, '..', '..', '..', 'database');
  if (!existsSync(ddlDir)) throw new Error(`spawnE2eDaemon: DDL dir missing: ${ddlDir}`);
  // `start` refuses when serve.pid is alive and cleans a stale one, so a
  // SIGKILLed predecessor never blocks us; execFileSync throws loudly on a
  // refusal instead of decaying into a silent 180s health wait.
  execFileSync(bin, ['start', '--instance', 'e2e'], {
    env: {
      ...process.env,
      SENSEI_INSTANCE: 'e2e',
      SENSEI_DDL_DIR: ddlDir,
      ...isolatedHomeEnvVars(),
    },
    stdio: 'pipe',
  });
}

async function restartDaemon(): Promise<void> {
  await killDaemon();
  await spawnE2eDaemon();
  await waitFor(async () => {
    try {
      const r = await fetch(`${DAEMON_URL}/health`);
      return r.ok;
    } catch {
      return false;
    }
  }, 180_000, 1000);
}

interface QueueTotals {
  totals: { pending: number; blocked: number; running: number };
}

async function waitForSettle(timeoutMs = 240_000): Promise<void> {
  await waitFor(async () => {
    const q = await daemonGet<QueueTotals>('/api/tasks/queue?limit=1');
    return q.totals.pending === 0 && q.totals.blocked === 0 && q.totals.running === 0;
  }, timeoutMs, 1000);
}

test.describe('Crash recovery matrix', () => {
  test.beforeEach(async ({ tauriPage }) => {
    test.setTimeout(420_000);
    await seedHealth(tauriPage);
    await seedSetupComplete(tauriPage);
    await installErrorTrap(tauriPage);
  });

  test.afterEach(async ({ tauriPage }, testInfo) => {
    // On failure, surface the webview's own errors — a blank screen with a
    // live socket reads identically with and without them.
    if (testInfo.status !== 'passed') {
      try {
        const errs = await readErrors(tauriPage);
        console.log(`[e2e-errors] console=${JSON.stringify(errs.console.slice(-5))}`);
        console.log(`[e2e-errors] error=${JSON.stringify(errs.error.slice(-5))}`);
        console.log(`[e2e-errors] rejection=${JSON.stringify(errs.rejection.slice(-5))}`);
      } catch { /* harness already gone */ }
    }
  });

  test('4.1 index crash resumes without redo (3.6)', async () => {
    const corpus = mkdtempSync(join(tmpdir(), 'sensei-e2e-crash41-'));
    const files: Record<string, string> = {};
    for (let i = 0; i < 40; i++) files[`src/mod${i}.py`] = `VALUE_${i} = ${i}\n`;
    makeGitRepo(join(corpus, 'repo'), files);
    const capture = captureSse(`${DAEMON_URL}/api/tasks/progress`);
    try {
      await daemonPost('/api/scan/roots', { path: corpus, excluded: [] });
      // Wait for the scan to actually start before crashing it.
      await capture.waitFor((e) => {
        const d = e.data as Record<string, unknown> | null;
        return !!d && d.entity === 'job' && d.action === 'add';
      }, 60_000);
      await restartDaemon();
      await waitForSettle();
      // No redo: each of the 40 files completed exactly once across both
      // runs — parsed pre-crash files were skipped, not re-parsed.
      const counts = psqlE2e(
        `SELECT path || '=' || count(*) FROM activity.task_executions
          WHERE path LIKE '${corpus}/repo/src/mod%.py' AND status = 'completed' AND task_kind = 'process_file'
          GROUP BY path`,
      )
        .trim()
        .split('\n')
        .filter(Boolean);
      expect(counts, 'all 40 files completed').toHaveLength(40);
      for (const line of counts) {
        expect(line, `${line} completed exactly once`).toMatch(/=1$/);
      }
    } finally {
      capture.close();
      await waitForSettle(60_000).catch(() => {});
      await bestEffort(() => removeRootsUnder(corpus));
    }
  });

  test('4.2 wave crash abandons and re-drives (3.2)', async () => {
    // Transcript fixtures are NEVER hand-written: fan the anonymised claude
    // fixture out to 30 sessions so the wave is still open when we crash it.
    // Re-drive runs through the backfill endpoint — the same dispatcher the
    // scheduler boot tick enqueues on non-isolated instances (this isolated
    // instance skips the tick by design, so the endpoint stands in for it).
    const projects = join(E2E_HOME, '.claude', 'projects', 'e2e-crash42');
    mkdirSync(projects, { recursive: true });
    const here = dirname(fileURLToPath(import.meta.url));
    const fixture = join(here, '..', '..', '..', 'crates/senseid/tests/fixtures/transcripts/claude/session-with-tools.jsonl');
    for (let i = 0; i < 30; i++) copyFileSync(fixture, join(projects, `e2e-session-${i}.jsonl`));
    // Ingestion is scoped (#125): a session with no row and no cwd under a
    // tracked folder is ignored, and the fixture carries no cwds. Seed the
    // rows — what hooks attribution leaves for a real session — so the
    // backfill ingests instead of scope-skipping every file.
    psqlE2e(
      `INSERT INTO activity.sessions(client_session_id) VALUES ${Array.from({ length: 30 }, (_, i) => `('e2e-session-${i}')`).join(', ')}`,
    );
    // Captures die with the daemon (no reconnect): one before the crash
    // for totals, a fresh one after for the re-driven terminal.
    const preCapture = captureSse(`${DAEMON_URL}/api/tasks/progress`);
    let postCapture: ReturnType<typeof captureSse> | null = null;
    try {
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
      // Totals mean the wave row is open — crash NOW, deterministically open.
      await preCapture.waitFor((e) => {
        const d = e.data as Record<string, unknown> | null;
        return !!d && d.entity === 'transcript' && d.action === 'add' && d.key === waveKey;
      }, 120_000);
      preCapture.close();
      await restartDaemon();
      postCapture = captureSse(`${DAEMON_URL}/api/tasks/progress`);
      // The dead wave reads abandoned, never lingering open...
      await waitFor(async () => {
        const all = (await (await fetch(`${DAEMON_URL}/api/tasks/waves?wave=transcript&limit=50`)).json()) as {
          waves: Array<{ key: string; outcome: string }>;
        };
        return all.waves.some((w) => w.key === waveKey && w.outcome === 'abandoned');
      }, 60_000, 1000);
      // ...and a fresh backfill covers every session exactly once: the
      // per-file upsert converges instead of duplicating.
      const redrive = (await (
        await fetch(`${DAEMON_URL}/api/transcripts/backfill`, { method: 'POST' })
      ).json()) as { taskId?: number };
      expect(typeof redrive.taskId, 're-drive enqueues').toBe('number');
      await postCapture!.waitFor((e) => {
        const d = e.data as Record<string, unknown> | null;
        return !!d && d.entity === 'transcript' && d.done === true && d.key === String(redrive.taskId);
      }, 240_000);
      const sessions = psqlE2e(
        `SELECT count(DISTINCT session_id) FROM activity.transcript_turns WHERE session_id LIKE 'e2e-session-%'`,
      ).trim();
      expect(sessions, 'all 30 sessions ingested across both waves').toBe('30');
    } finally {
      preCapture.close();
      postCapture?.close();
      await fetch(`${DAEMON_URL}/api/transcripts/consent/claude_code`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ consented: false }),
      }).catch(() => {});
      rmSync(projects, { recursive: true, force: true });
      psqlE2e(`DELETE FROM activity.transcript_turns WHERE session_id LIKE 'e2e-session-%'`);
      psqlE2e(`DELETE FROM activity.sessions WHERE client_session_id LIKE 'e2e-session-%'`);
      psqlE2e(`DELETE FROM activity.capture_watermarks WHERE file_path LIKE '%e2e-crash42%'`);
    }
  });

  test('4.3 orphaned tasks return with lineage; the drill-down shows the chain', async ({
    tauriPage,
  }) => {
    psqlE2e(
      `INSERT INTO activity.task_executions(task_id, task_kind, folder_path, path, status, retry_number, started_at) VALUES
       (330101, 'process_file', '/r', '/r/crash-orphan.ts', 'running', 1, now() - interval '1 hour')`,
    );
    try {
      await restartDaemon();
      // The orphan reconciled, then returned: find the new attempt by lineage.
      let newId = '';
      await waitFor(async () => {
        const prior = psqlE2e(
          `SELECT id FROM activity.task_executions WHERE task_id = 330101 ORDER BY started_at DESC LIMIT 1`,
        ).trim();
        newId = psqlE2e(
          `SELECT task_id FROM activity.task_executions WHERE retried_from_execution_id = '${prior}' LIMIT 1`,
        ).trim();
        return newId !== '';
      }, 90_000, 2000);
      // And the center shows the chain.
      await navigateToScreen(tauriPage, '/operations', '[data-component="ops-alerts"]');
      await tauriPage.waitForSelector('[data-component="ops-failures"]', 30_000);
      const expand = `[data-failure-row="${newId}"]`;
      await waitFor(async () => (await tauriPage.locator(expand).count()) > 0, 90_000, 2000);
      await tauriPage.locator(expand).click();
      await waitFor(async () => {
        const text = await tauriPage.locator(`[data-failure-detail="${newId}"]`).textContent();
        return (text ?? '').includes('retried from #330101');
      }, 30_000, 500);
    } finally {
      // Path-scoped: covers the retried rows too, whatever IDs they took.
      psqlE2e(`DELETE FROM activity.task_executions WHERE path = '/r/crash-orphan.ts'`);
    }
  });

  test('4.4 acks survive restart; recurrence escalates across restart', async ({ tauriPage }) => {
    // Same inbound hygiene as 9.5: a disposition left by a killed run would
    // render the alert pre-acked and fail the ack step.
    psqlE2e(`DELETE FROM activity.alert_acks WHERE class = 'task' AND alert_key = '330201'`);
    psqlE2e(
      `INSERT INTO activity.task_executions(task_id, task_kind, folder_path, path, status, error_message) VALUES
       (330201, 'process_file', '/r', '/r/ack-crash.ts', 'failed', 'e2e ack crash')`,
    );
    try {
      await navigateToScreen(tauriPage, '/operations', '[data-component="ops-alerts"]');
      await tauriPage.waitForSelector('[data-component="ops-alerts"]', 30_000);
      await waitFor(async () => (await tauriPage.locator('[data-alert-ack="task:330201"]').count()) > 0, 60_000, 1000);
      await tauriPage.locator('[data-alert-ack="task:330201"]').click();
      await waitFor(async () => {
        const text = await tauriPage.locator('[data-component="ops-alerts"]').textContent();
        return (text ?? '').includes('Acked');
      }, 30_000, 500);

      await restartDaemon();

      // Still acked after the restart — dispositions persisted.
      await navigateToScreen(tauriPage, '/operations', '[data-component="ops-alerts"]');
      await tauriPage.waitForSelector('[data-component="ops-alerts"]', 30_000);
      await waitFor(async () => {
        const text = await tauriPage.locator('[data-component="ops-alerts"]').textContent();
        return (text ?? '').includes('Acked');
      }, 60_000, 1000);

      // Clear, then recur: the recurrence escalates from durable memory.
      psqlE2e('DELETE FROM activity.task_executions WHERE task_id = 330201');
      await tauriPage.locator('[data-failures-refresh]').click();
      await navigateToScreen(tauriPage, '/operations', '[data-component="ops-alerts"]');
      await waitFor(async () => (await tauriPage.locator('[data-alert-ack="task:330201"]').count()) === 0, 60_000, 1000);
      // The clear write is async — wait for it to land before recurring,
      // or the re-derive reads stale memory and the test flakes.
      await waitFor(async () => {
        const n = psqlE2e(
          `SELECT seen_clears FROM activity.alert_acks WHERE class = 'task' AND alert_key = '330201'`,
        ).trim();
        return n !== '' && Number(n) >= 1;
      }, 30_000, 500);
      psqlE2e(
        `INSERT INTO activity.task_executions(task_id, task_kind, folder_path, path, status, error_message) VALUES
         (330201, 'process_file', '/r', '/r/ack-crash.ts', 'failed', 'e2e ack crash again')`,
      );
      // Failures re-read on explicit refresh only — without it the alerts
      // keep deriving from the cleared feed and the recurrence never shows.
      await tauriPage.locator('[data-failures-refresh]').click();
      await navigateToScreen(tauriPage, '/operations', '[data-component="ops-alerts"]');
      await waitFor(async () => {
        const text = await tauriPage.locator('[data-component="ops-alerts"]').textContent();
        return (text ?? '').includes('Escalated');
      }, 60_000, 1000);
    } finally {
      psqlE2e('DELETE FROM activity.task_executions WHERE task_id = 330201');
      psqlE2e(`DELETE FROM activity.alert_acks WHERE class = 'task' AND alert_key = '330201'`);
    }
  });

  test('4.5 boot backfills fire after restart; stale schedule state clears (3.3/3.4/3.7)', async () => {
    const corpus = mkdtempSync(join(tmpdir(), 'sensei-e2e-crash45-'));
    makeGitRepo(join(corpus, 'repo'), { 'src/m.py': 'M = 1\n' });
    const patched: Array<{ name: string; interval_secs: number }> = [];
    try {
      await daemonPost('/api/scan/roots', { path: corpus, excluded: [] });
      await waitForSettle();
      // Shorten cadences BEFORE the crash so scheduled passes fire soon
      // after the reboot, when the capture is already listening. (Boot
      // passes fire before any capture could connect, so they are proven
      // via wave_runs rows, not the stream.)
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
      // A stale pass-in-progress from the dead session.
      psqlE2e(`UPDATE sensei.schedules SET running_since = now() - interval '1 hour' WHERE name = 'dojo_sync'`);
      const t0 = new Date().toISOString();
      await restartDaemon();
      // Capture AFTER the restart: captures die with the daemon.
      const capture = captureSse(`${DAEMON_URL}/api/tasks/progress`);
      try {
        // Stale running state clears at boot (#283)...
        await waitFor(async () => {
          const after = await daemonGet<{ tasks: Array<{ name: string; running_since: string | null }> }>(
            '/api/tasks/scheduled',
          );
          return after.tasks.find((t) => t.name === 'dojo_sync')?.running_since === null;
        }, 60_000, 1000);
        // ...the boot backfills ran (3.3/3.4): done rows opened after t0...
        await waitFor(async () => {
          const all = (await (await fetch(`${DAEMON_URL}/api/tasks/waves?limit=50`)).json()) as {
            waves: Array<{ wave: string; outcome: string; opened_at: string }>;
          };
          const fresh = all.waves.filter((w) => w.opened_at >= t0 && w.outcome === 'done');
          return fresh.some((w) => w.wave === 'sync') && fresh.some((w) => w.wave === 'metric');
        }, 180_000, 2000);
        // ...and the live stream still flows post-restart.
        await capture.waitFor((e) => {
          const d = e.data as Record<string, unknown> | null;
          return !!d && d.entity === 'sync' && d.done === true;
        }, 180_000);
      } finally {
        capture.close();
      }
    } finally {
      for (const { name, interval_secs } of patched) {
        await fetch(`${DAEMON_URL}/api/tasks/scheduled/${name}`, {
          method: 'PATCH',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ interval_secs }),
        }).catch(() => {});
      }
      await waitForSettle(60_000).catch(() => {});
      await bestEffort(() => removeRootsUnder(corpus));
    }
  });
});
