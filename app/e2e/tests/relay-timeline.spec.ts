/**
 * Relay timeline + steps (F8 exit proof): a full crew lifecycle against the
 * fresh e2e broker, asserting the checklist and timeline update over the SSE
 * stream with NO manual refresh.
 *
 * Unlike relay.spec.ts (seed-agnostic), this spec mints its own crew + lanes
 * and drives them over the lane HTTP API: register, plan, assign, post step
 * updates, report, review, merge. The app sits on the task view the whole
 * time; every assertion after the first navigation proves a stream delivery.
 */
import { test, expect, type TauriPageLike } from '../fixtures';
import { navigateTo, navigateToScreen, DAEMON_URL } from '../helpers';

const TITLE = 'F8 e2e checkout counter';
const STREAM_TIMEOUT = 15_000;

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

interface Lane {
  lane_id: string;
  lane_token: string;
}

async function relayPost<T>(path: string, token: string | null, body: unknown): Promise<T> {
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

async function relayGet<T>(path: string, token: string): Promise<T> {
  const resp = await fetch(`${DAEMON_URL}${path}`, {
    headers: { Authorization: `Bearer ${token}` },
  });
  if (!resp.ok) throw new Error(`GET ${path}: HTTP ${resp.status}: ${await resp.text()}`);
  return (await resp.json()) as T;
}

async function registerLane(role: string, crew_id?: string): Promise<Lane & { crew_id: string }> {
  return relayPost('/api/relay/register', null, {
    role,
    family: 'e2e',
    goal: 'F8 e2e crew',
    ...(crew_id ? { crew_id } : {}),
  });
}

test.describe('Relay — timeline + steps over the live stream', () => {
  test.beforeEach(async ({ tauriPage }) => {
    await seedSetupComplete(tauriPage);
    await navigateTo(tauriPage, '/logs'); // reset the SPA before navigating to the target
  });

  test('checklist and timeline update with no refresh', async ({ tauriPage }) => {
    test.setTimeout(180_000);

    // The crew: a coordinator minting, a contributor holding, a reviewer watching.
    const coord = await registerLane('coordinator');
    const crew = coord.crew_id;
    const holder = await registerLane('contributor', crew);
    const reviewer = await registerLane('contributor', crew);

    const { task_id: task } = await relayPost<{ task_id: string }>(
      '/api/relay/tasks/plan',
      coord.lane_token,
      { lane_id: coord.lane_id, title: TITLE, phase: 'impl', repo_key: 'sensei' },
    );
    await relayPost(`/api/relay/tasks/${task}/assign`, coord.lane_token, {
      lane_id: coord.lane_id,
      to_lane: holder.lane_id,
      title: TITLE,
      phase: 'impl',
      brief: { summary: 'Ring up the e2e groceries' },
    });
    // Delivery starts the task: the holder's envelope pull flips assigned →
    // working (no start tool exists), which is when steps become postable.
    const envelope = await relayGet<{ assignments: Array<{ task_id: string }> }>(
      `/api/relay/tasks?lane_id=${holder.lane_id}`,
      holder.lane_token,
    );
    if (!envelope.assignments.some((a) => a.task_id === task)) {
      throw new Error('holder envelope carries no assignment for the task');
    }
    await relayPost(`/api/relay/tasks/${task}/steps`, holder.lane_token, {
      lane_id: holder.lane_id,
      key: 'scaffold',
      title: 'Scaffold the counter',
      state: 'open',
    });

    // The person opens the task view and stays there for the rest of the test.
    await navigateToScreen(
      tauriPage,
      `/relay/${crew}/${task}`,
      '[data-screen="relay"] [data-task-title]',
    );
    await expect(tauriPage.locator('[data-task-title]')).toContainText(TITLE);
    await expect(tauriPage.locator('[data-step="scaffold"]')).toHaveAttribute(
      'data-step-state',
      'open',
    );
    await expect(
      tauriPage.locator('[data-timeline-toggle]').filter({ hasText: 'Assigned' }),
    ).toBeVisible();

    // Step progress streams in: scaffold done, tests now current. No navigation.
    await relayPost(`/api/relay/tasks/${task}/steps`, holder.lane_token, {
      lane_id: holder.lane_id,
      key: 'scaffold',
      title: 'Scaffold the counter',
      state: 'done',
    });
    await relayPost(`/api/relay/tasks/${task}/steps`, holder.lane_token, {
      lane_id: holder.lane_id,
      key: 'tests',
      title: 'Ring up the tests',
      state: 'open',
    });
    await expect(tauriPage.locator('[data-step="scaffold"]')).toHaveAttribute(
      'data-step-state',
      'done',
      { timeout: STREAM_TIMEOUT },
    );
    await expect(tauriPage.locator('[data-step="tests"]')).toBeVisible({
      timeout: STREAM_TIMEOUT,
    });
    await expect(tauriPage.locator('[data-step="tests"]')).toHaveAttribute(
      'data-step-current',
      'true',
      { timeout: STREAM_TIMEOUT },
    );

    // The full report lands as a timeline entry and moves the state — still
    // no refresh.
    // The report names a branch, so approval holds the task for the merge
    // below (a report with nothing to merge ends it done at once).
    await relayPost(`/api/relay/tasks/${task}/report`, holder.lane_token, {
      lane_id: holder.lane_id,
      phase: 'impl',
      round: 1,
      summary: { changed: 'Counter scaffolds; tests ring true' },
      branch: 'lane/counter',
    });
    await expect(
      tauriPage
        .locator('[data-timeline-toggle]')
        .filter({ hasText: 'Reported — Work · impl r1' }),
    ).toBeVisible({ timeout: STREAM_TIMEOUT });
    await expect(tauriPage.locator('[data-task-state]')).toContainText('in review', {
      timeout: STREAM_TIMEOUT,
    });

    // Thin rows expand to the full entry the History renders today. Only the
    // clicked row renders a detail, so the bare locator matches exactly one —
    // no .first() on the assertion path (tauri-playwright-testing §6.1).
    await expect(tauriPage.locator('[data-timeline] [data-timeline-detail]')).toHaveCount(
      0,
    );
    await tauriPage.locator('[data-timeline-toggle]').first().click();
    await expect(
      tauriPage.locator('[data-timeline] [data-timeline-detail]'),
    ).toHaveCount(1);
    await expect(
      tauriPage.locator('[data-timeline] [data-timeline-detail]'),
    ).toBeVisible();

    // The report floats on the review queue; the reviewer claims it, then
    // review + merge close the lifecycle on the same unmoved screen.
    await relayPost(`/api/relay/tasks/${task}/claim_review`, reviewer.lane_token, {
      lane_id: reviewer.lane_id,
    });
    await relayPost(`/api/relay/tasks/${task}/review`, reviewer.lane_token, {
      lane_id: reviewer.lane_id,
      verdict: 'approved',
      summary: 'Counter rings true',
    });
    await expect(
      tauriPage.locator('[data-timeline-toggle]').filter({ hasText: 'Review approved' }),
    ).toBeVisible({
      timeout: STREAM_TIMEOUT,
    });
    await expect(tauriPage.locator('[data-task-state]')).toContainText('approved', {
      timeout: STREAM_TIMEOUT,
    });

    await relayPost(`/api/relay/tasks/${task}/merge`, coord.lane_token, {
      lane_id: coord.lane_id,
      into: 'develop',
      note: 'F8 e2e merge',
    });
    await expect(
      tauriPage
        .locator('[data-timeline-toggle]')
        .filter({ hasText: 'Merged into develop' }),
    ).toBeVisible({ timeout: STREAM_TIMEOUT });
    await expect(tauriPage.locator('[data-task-state]')).toContainText('done', {
      timeout: STREAM_TIMEOUT,
    });
  });
});
