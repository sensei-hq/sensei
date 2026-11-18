/**
 * A relay decision answered with a form (#389), against the e2e broker.
 *
 * Slice A — the wire: a lane asks the person with a `response` (a flat JSON Schema
 * subset); `GET /relay/now` lists it with that response and no options; the daemon
 * refuses values that break the schema (422 naming the field, nothing recorded); the
 * person answers with valid values; the asking lane's envelope reads them back
 * exactly, with `option` null. The option-pick journey stays in relay-run.spec.ts.
 *
 * The task view reads a form answer back as its values, in the timeline row and its
 * expanded entry — never "null" or an empty option.
 *
 * The screen tests (the form drawn, live arrival and resolution) join this file in
 * slice B.
 */
import { test, expect } from '../fixtures';
import { DAEMON_URL, navigateTo, navigateToScreen } from '../helpers';
import { RelayCrew, type RelayLane } from '../lib/relay-crew';

const RUN_TAG = Date.now().toString(36);

interface DecisionRow {
  id: string;
  options: unknown[];
  response: unknown;
}

/** What the routes this spec reads answer with (only the fields it checks). */
interface Answered {
  decisions: DecisionRow[];
  message: string;
}

/** A person route on the daemon (no lane token): its status and body, refusals included. */
async function person(method: string, path: string, body?: unknown): Promise<{ status: number; body: Answered }> {
  const res = await fetch(`${DAEMON_URL}${path}`, {
    method,
    headers: { 'Content-Type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const text = await res.text();
  return { status: res.status, body: text ? JSON.parse(text) : null };
}

const RESPONSE = {
  schema: {
    type: 'object',
    properties: {
      target: { type: 'string', enum: ['develop', 'main'], title: 'Target branch' },
      count: { type: 'integer', minimum: 1, maximum: 5 },
      ship: { type: 'boolean', description: 'Ship it now?' },
    },
    required: ['target'],
  },
};

test.describe('relay decision form (#389)', () => {
  let lane: RelayLane | null = null;

  test.afterAll(async () => {
    if (!lane) return;
    await person('POST', `/relay/crews/${lane.crewId}/close`, {});
  });

  test('the broker round-trips a form decision', async () => {
    lane = await new RelayCrew(DAEMON_URL).register({
      role: 'coordinator',
      family: 'e2e',
      goal: `decision form ${RUN_TAG}`,
      repo_key: 'github.com/sensei-hq/e2e-form',
    });
    const decisionId = await lane.ask({ question: `Where does it ship? ${RUN_TAG}`, response: RESPONSE });

    // Now lists it with its response and no buttons.
    const now = await person('GET', '/relay/now');
    expect(now.status).toBe(200);
    const row = now.body.decisions.find((d) => d.id === decisionId);
    expect(row, 'the form decision is on Now').toBeTruthy();
    expect(row?.response).toEqual(RESPONSE);
    expect(row?.options).toEqual([]);

    // Values that break the schema are refused by name, and nothing is recorded.
    const bad = await person('POST', '/relay/answers', {
      decision_id: decisionId,
      values: { target: 'main', count: 9 },
      note: null,
      surface: 'desktop',
    });
    expect(bad.status).toBe(422);
    expect(bad.body.message).toContain('values.count');
    const missing = await person('POST', '/relay/answers', { decision_id: decisionId, values: { ship: true }, note: null, surface: 'desktop' });
    expect(missing.status).toBe(422);
    expect(missing.body.message).toContain('values.target is required');
    expect((await lane.envelope()).answers.filter((a) => a.decision_id === decisionId)).toEqual([]);

    // The person answers; the asking lane reads the exact values, option null.
    const values = { target: 'main', count: 3, ship: true };
    const ok = await person('POST', '/relay/answers', { decision_id: decisionId, values, note: null, surface: 'desktop' });
    expect(ok.status, JSON.stringify(ok.body)).toBe(200);
    const answer = (await lane.envelope()).answers.find((a) => a.decision_id === decisionId);
    expect(answer).toMatchObject({ decision_id: decisionId, option: null, values });

    // Answered: it leaves Now.
    const after = await person('GET', '/relay/now');
    expect(after.body.decisions.some((d) => d.id === decisionId)).toBe(false);
  });

  test('the task view reads a form answer as its values', async ({ tauriPage }) => {
    test.setTimeout(120_000);
    await fetch(`${DAEMON_URL}/api/config`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ setup_complete: '1' }),
    });
    await navigateTo(tauriPage, '/logs'); // reset the SPA before navigating to the target

    const coord = await new RelayCrew(DAEMON_URL).register({
      role: 'coordinator',
      family: 'e2e',
      goal: `decision form task ${RUN_TAG}`,
      repo_key: 'github.com/sensei-hq/e2e-form',
    });
    lane ??= coord;
    const title = `Form answer ${RUN_TAG}`;
    const taskId = await coord.plan({ title, phase: 'impl', repo_key: 'github.com/sensei-hq/e2e-form' });
    const decisionId = await coord.ask({ task_id: taskId, question: `Where does it ship? ${RUN_TAG}`, response: RESPONSE });
    const values = { target: 'main', count: 3, ship: true };
    const ok = await person('POST', '/relay/answers', { decision_id: decisionId, values, note: null, surface: 'desktop' });
    expect(ok.status, JSON.stringify(ok.body)).toBe(200);

    await navigateToScreen(tauriPage, `/relay/${coord.crewId}/${taskId}`, '[data-screen="relay"] [data-task-title]');
    await expect(tauriPage.locator('[data-task-title]')).toContainText(title);
    const row = tauriPage.locator('[data-timeline-toggle]').filter({ hasText: 'Answered:' });
    await expect(row).toContainText('Answered: count: 3, ship: true, target: main');
    await row.click();
    await expect(tauriPage.locator('[data-relay-timeline] [data-timeline-detail] [data-entry-answer]')).toContainText(
      'count: 3, ship: true, target: main · from the desktop',
    );

    if (lane !== coord) await person('POST', `/relay/crews/${coord.crewId}/close`, {});
  });
});
