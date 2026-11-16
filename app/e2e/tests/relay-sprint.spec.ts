/**
 * Relay sprint band + floating reviews (queue cutover, phase 4 exit proof):
 * the person creates a sprint run over HTTP while the Relay screen is open,
 * and the band appears with NO manual refresh; a lane claims Task 0 and
 * reports, and the floating-review row appears the same way. Every
 * assertion after the first navigation proves a stream delivery.
 *
 * Mirrors relay-timeline.spec.ts: mints its own crew and lane against the
 * fresh e2e broker and drives them over the lane HTTP API.
 */
import { test, expect, type TauriPageLike } from '../fixtures';
import { navigateTo, navigateToScreen, DAEMON_URL } from '../helpers';

const STREAM_TIMEOUT = 15_000;
const REPO = 'github.com/sensei-hq/e2e-sprint';

async function seedSetupComplete(tauriPage: TauriPageLike): Promise<void> {
  await fetch(`${DAEMON_URL}/api/config`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ setup_complete: '1' }),
  });
  await tauriPage.evaluate(`
    (function() {
      var s = window.__sensei_state__;
      if (s && s.appState) {
        s.appState.config = Object.assign({}, s.appState.config, { setup_complete: '1' });
        s.appState.loaded = true;
      }
    })()
  `);
}

async function post<T>(path: string, token: string | null, body: unknown): Promise<T> {
  const resp = await fetch(`${DAEMON_URL}${path}`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
    },
    body: JSON.stringify(body),
  });
  if (!resp.ok) throw new Error(`POST ${path}: HTTP ${resp.status}: ${await resp.text()}`);
  return (await resp.json()) as T;
}

interface Lane {
  lane_id: string;
  lane_token: string;
  crew_id: string;
}

test.describe('Relay — sprint band and floating reviews over the live stream', () => {
  test.beforeEach(async ({ tauriPage }) => {
    await seedSetupComplete(tauriPage);
    await navigateTo(tauriPage, '/logs');
  });

  test('a run and a floating review appear with no refresh', async ({ tauriPage }) => {
    test.setTimeout(180_000);

    // A crew with a repository (a run needs one), and one contributor lane that
    // joins by crew_id (other crews may be open on the e2e broker).
    const coord = await post<Lane>('/api/relay/register', null, {
      role: 'coordinator',
      family: 'e2e',
      goal: 'sprint e2e crew',
      repo_key: REPO,
    });
    const worker = await post<Lane>('/api/relay/register', null, {
      family: 'e2e',
      crew_id: coord.crew_id,
      suggested_name: 'worker-e2e',
    });
    expect(worker.crew_id).toBe(coord.crew_id);

    // The person opens Relay and stays there for the rest of the test.
    await navigateToScreen(tauriPage, '/relay', '[data-screen="relay"] [data-relay-section="lanes"]');
    await expect(tauriPage.locator('[data-lane-row]').filter({ hasText: 'worker-e2e' })).toBeVisible();

    // A triage item, then the run over HTTP: the band streams in.
    const { item_id } = await post<{ item_id: string }>('/relay/triage', null, {
      title: 'Sprint e2e gap',
      kind: 'gap',
      repo_key: REPO,
      tags: [],
    });
    const run = await post<{ run_id: string; planning_task_id: string }>('/relay/runs', null, {
      crew_id: coord.crew_id,
      title: 'Sprint e2e',
      goal: 'Prove the band',
      item_ids: [item_id],
      repo_key: REPO,
      config: { auto_merge: false },
      start: true,
    });
    const band = tauriPage.locator(`[data-sprint-band][data-run-id="${run.run_id}"]`);
    await expect(band).toBeVisible({ timeout: STREAM_TIMEOUT });
    await expect(band.locator('[data-state]')).toHaveText('planning · Task 0 in the queue');

    // The worker claims Task 0 and reports its plan: the review floats in.
    const claim = await post<{ claim_type: string; task: { id: string } }>(
      '/api/relay/tasks/claim_next',
      worker.lane_token,
      { lane_id: worker.lane_id },
    );
    expect(claim.claim_type).toBe('task');
    expect(claim.task.id).toBe(run.planning_task_id);
    const plan = {
      tasks: [
        { key: 't1', title: 'First e2e file', phase: 'impl', owned_paths: ['t1.txt'], acceptance: ['t1.txt exists'], deps: [] },
      ],
    };
    await post(`/api/relay/tasks/${run.planning_task_id}/report`, worker.lane_token, {
      lane_id: worker.lane_id,
      phase: 'planning',
      round: 1,
      summary: { changed: 'Planned one file.' },
      report_md: `Plan:\n\n\`\`\`json\n${JSON.stringify(plan)}\n\`\`\`\n`,
    });
    const review = tauriPage.locator(`[data-floating-review][data-task-id="${run.planning_task_id}"]`);
    await expect(review).toBeVisible({ timeout: STREAM_TIMEOUT });
    await expect(review.locator('[data-author]')).toContainText('worker-e2e');
    await expect(band.locator('[data-count="in_review"]')).toContainText('1 floating', { timeout: STREAM_TIMEOUT });
  });
});
