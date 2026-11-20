/**
 * A crew whose contributor died closes from the coordinator lane (#398).
 *
 * A session that dies never unregisters: its lane stays registered with an
 * expired lease. The lane close counts a lane as live by its lease, so the dead
 * contributor does not block it; close releases it. The screen is opened once,
 * and both lane rows leave it from the stream — no reload, no navigation.
 */
import { execFileSync } from 'child_process';
import { test, expect } from '../fixtures';
import { DAEMON_URL, navigateToScreen, seedSetupComplete } from '../helpers';
import { psqlBin } from '../lib/psql';
import { RelayCrew, RelayLane } from '../lib/relay-crew';

const E2E_DB = 'sensei_e2e';
const STREAM = 20_000;

async function person<T>(method: string, path: string, body?: unknown): Promise<T> {
  const res = await fetch(`${DAEMON_URL}${path}`, {
    method,
    headers: { 'Content-Type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  if (!res.ok) throw new Error(`${method} ${path}: HTTP ${res.status}: ${await res.text()}`);
  return (await res.json()) as T;
}

/** Earlier specs leave crews open on the shared e2e broker: end them (and their runs). */
async function cleanSlate(): Promise<void> {
  const runs = await person<{ active: Array<{ id: string }> }>('GET', '/relay/runs');
  for (const run of runs.active) await person('POST', `/relay/runs/${run.id}/cancel`, {});
  const now = await person<{ crews: Array<{ id: string }> }>('GET', '/relay/now');
  for (const crew of now.crews) await person('POST', `/relay/crews/${crew.id}/close`, {});
}

/** The contributor's session died: its lane lease ends a minute ago, in the e2e DB only. */
async function killLease(laneId: string): Promise<void> {
  const health = (await fetch(`${DAEMON_URL}/health`).then((r) => r.json())) as { dbName?: string };
  if (health.dbName !== E2E_DB) throw new Error(`refusing to back-date a lease on "${health.dbName}"`);
  if (!/^[0-9a-f-]{36}$/.test(laneId)) throw new Error(`not a lane id: ${laneId}`);
  const sql = `update relay.leases set expires_at = current_timestamp - interval '1 minute'
               where lane_id = '${laneId}' and kind = 'lane' and state = 'granted'`;
  execFileSync(psqlBin(), ['-d', E2E_DB, '-v', 'ON_ERROR_STOP=1', '-c', sql], { stdio: 'pipe' });
}

test('a crew with a dead contributor closes from the coordinator; its lanes leave the screen', async ({ tauriPage }) => {
  test.setTimeout(180_000);
  await cleanSlate();
  const crew = new RelayCrew(DAEMON_URL);
  const coord = await crew.register({ role: 'coordinator', family: 'e2e', goal: 'close e2e crew', suggested_name: 'close-coord-e2e' });
  const worker = await crew.register({ family: 'e2e', crew_id: coord.crewId, suggested_name: 'close-worker-e2e' });

  await seedSetupComplete(tauriPage);
  await navigateToScreen(tauriPage, '/relay', '[data-screen="relay"] [data-relay-section="lanes"]');
  const row = (name: string) => tauriPage.locator('[data-lane-row]').filter({ hasText: name });
  await expect(row('close-coord-e2e')).toBeVisible({ timeout: STREAM });
  await expect(row('close-worker-e2e')).toBeVisible({ timeout: STREAM });

  await killLease(worker.id);
  const closed = await RelayLane.attempt(() => coord.closeCrew());
  expect(closed, JSON.stringify(closed)).toMatchObject({ ok: true, value: { status: 'closed', released: [worker.id] } });

  // Both rows leave from the stream: the released lane by its own event, the
  // coordinator's with the closed crew.
  await expect(row('close-worker-e2e')).toBeHidden({ timeout: STREAM });
  await expect(row('close-coord-e2e')).toBeHidden({ timeout: STREAM });
  const now = await person<{ crews: Array<{ id: string }> }>('GET', '/relay/now');
  expect(now.crews.map((c) => c.id)).not.toContain(coord.crewId);

  // The dead session comes back: its lane is gone and the crew is closed to it.
  const beat = await RelayLane.attempt(() => worker.heartbeat());
  expect(beat).toMatchObject({ ok: false, status: 410, error: 'relay_lane_gone' });
  await coord.unregister();
});
