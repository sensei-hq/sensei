// @vitest-environment jsdom
// Workspace · Relay · a task — the task view (CONTRACT §1e; `relay.task_history`).
//
// The loader and the page are imported inside each test. The relay client is mocked
// (`$lib/relay-api`, `relayApi(port)`), as on the Now screen.
//
// What this pins beyond the layout: the history keeps the view's order (newest
// first, down to Assigned); a message's delivery receipts fold into the message
// instead of standing as entries; and a number nobody reported reads "not
// reported", while a reported zero reads 0 (D83).
import { describe, it, expect, afterEach, vi } from 'vitest';
import { tick } from 'svelte';
import { mountComponent } from '$lib/test-mount.js';
import type { ApiResult } from '$lib/api.js';
import type { RelayAction, RelayTaskView } from '$lib/relay-api.js';
import * as F from '$lib/test-stubs/relay-fixtures.js';

const PAGE = './+page.svelte';
const LOADER = './+page.js';

const relay = vi.hoisted(() => ({ api: null as unknown }));
vi.mock('$lib/relay-api.js', () => ({ relayApi: vi.fn(() => relay.api) }));
// The socket is the route's, pinned in relay-task-stream.spec — not here.
vi.mock('$lib/relay/relay-task-stream.js', () => ({ subscribeTaskStream: vi.fn(() => () => {}) }));

const route = vi.hoisted(() => ({ id: 'w1', crew: '', task: 't-header' }));
vi.mock('$app/state', () => ({
  page: {
    get params() {
      return { id: route.id, crew: route.crew, task: route.task };
    },
    get url() {
      return new URL(`http://localhost/workspace/${route.id}/relay/${route.crew}/${route.task}`);
    },
  },
}));
vi.mock('$app/navigation', () => ({ goto: vi.fn(), invalidateAll: vi.fn() }));

vi.mock('@rokkit/app', async (importOriginal) => {
  const m = (await importOriginal()) as Record<string, unknown>;
  return { ...m, ScreenShell: vi.fn(m.ScreenShell as never) };
});

import { ScreenShell } from '@rokkit/app';

let cleanup: Array<() => void> = [];
afterEach(() => {
  cleanup.forEach((fn) => fn());
  cleanup = [];
  vi.unstubAllGlobals();
  vi.mocked(ScreenShell).mockClear();
  route.id = 'w1';
  route.crew = '';
  route.task = 't-header';
});

async function open(
  opts: {
    task?: ApiResult<RelayTaskView>;
    taskId?: string;
    crewId?: string;
    workspace?: string;
    /** Shape the fake before the loader reads (e.g. answer per workspace). */
    setup?: (api: F.FakeRelayApi) => void;
  } = {},
) {
  const api = F.fakeRelayApi();
  if (opts.task) api.tryGetTask.mockResolvedValue(opts.task);
  opts.setup?.(api);
  relay.api = api;
  route.id = opts.workspace ?? 'w1';
  route.crew = opts.crewId ?? F.CREW_ID;
  route.task = opts.taskId ?? 't-header';
  const { load } = await import(/* @vite-ignore */ LOADER);
  const data = await load({
    params: { id: route.id, crew: route.crew, task: route.task },
    url: new URL(`http://localhost/workspace/${route.id}/relay/${route.crew}/${route.task}`),
  } as never);
  const { default: Page } = await import(/* @vite-ignore */ PAGE);
  const m = mountComponent(Page, { data } as never);
  cleanup.push(m.destroy);
  await tick();
  return { c: m.container, api };
}

const one = (root: ParentNode, sel: string) => root.querySelector(sel) as HTMLElement | null;
const all = (root: ParentNode, sel: string) => [...root.querySelectorAll(sel)] as HTMLElement[];
const text = (el: Element | null | undefined) => (el?.textContent ?? '').replace(/\s+/g, ' ').trim();

/** The element, or a failure that names what is missing — never a null dereference,
 *  so a red says which part of the screen is not there yet. */
function need<T extends Element>(el: T | null | undefined, what: string): T {
  expect(el ?? null, `missing: ${what}`).not.toBeNull();
  return el as T;
}
const entry = (c: HTMLElement, id: string) => one(c, `[data-history-entry="${id}"]`);

describe('Relay · task — the loader’s read', () => {
  it('reads this task’s history, and the page reads nothing of its own', async () => {
    const fetchSpy = vi.fn();
    vi.stubGlobal('fetch', fetchSpy);
    const { api } = await open({ taskId: 't-billing', crewId: F.OTHER_CREW_ID, workspace: 'w2' });
    expect(api.tryGetTask).toHaveBeenCalledTimes(1);
    // r2 F3 (repository ruling): the read is scoped by the crew in the route, not the workspace.
    expect(api.tryGetTask).toHaveBeenCalledWith(F.OTHER_CREW_ID, 't-billing');
    expect(fetchSpy).not.toHaveBeenCalled();
  });
});

describe('Relay · task — the header', () => {
  it('rokkit’s shell for workspace.relay.task, mark 継, no stats band', async () => {
    const { c } = await open();
    expect(vi.mocked(ScreenShell)).toHaveBeenCalled();
    expect(vi.mocked(ScreenShell).mock.calls[0][1]).toMatchObject({
      viewId: 'workspace.relay.task',
      mark: { kanji: '継' },
    });
    expect(one(c, '[data-screen-stats]')).toBeNull();
    expect(one(c, '[data-screen="relay"]')).not.toBeNull();
  });

  it('back to Relay, the title, its state, and the ticket button (provider + key, external)', async () => {
    const { c } = await open({ workspace: 'w2' });
    expect(one(c, 'a[data-back]')?.getAttribute('href')).toBe('/workspace/w2/relay');
    expect(text(one(c, '[data-task-title]'))).toBe('Diagrams header on ScreenShell');
    expect(text(one(c, '[data-task-state]'))).toBe('done');
    const ticket = need(one(c, 'a[data-ticket]'), 'a[data-ticket]');
    expect(ticket.getAttribute('href')).toBe('https://acme.atlassian.net/browse/SEN-52');
    expect(ticket.getAttribute('target')).toBe('_blank');
    expect(text(one(ticket, '[data-ticket-provider]'))).toBe('Jira');
    expect(text(ticket)).toContain('SEN-52');
  });

  it('a task with no ticket has no ticket button', async () => {
    const view = F.taskView();
    const { c } = await open({ task: F.ok({ ...view, task: { ...view.task, ticket: null } }) });
    expect(one(c, '[data-task-title]')).not.toBeNull();
    expect(one(c, '[data-ticket]')).toBeNull();
  });

  it('one facts line: lane, member, assistant and model, and rounds by phase', async () => {
    const { c } = await open();
    const facts = text(one(c, '[data-task-facts]'));
    expect(facts).toContain('Diagrams');
    expect(facts).toContain('L1');
    expect(facts).toContain('2.1');
    expect(facts).toContain('claude-opus-5-5');
    expect(facts).toContain('tests 1');
    expect(facts).toContain('impl 2');
  });

  it('a limit-paused holder reads paused with scope and resume time', async () => {
    const resumes = new Date(Date.now() + 120 * 60_000).toISOString();
    const view = F.taskView();
    const { c } = await open({
      task: F.ok({
        ...view,
        task: {
          ...view.task,
          lane: { ...view.task.lane!, limit_scope: 'weekly', limit_resumes_at: resumes, limit_note: null },
        },
      }),
    });
    const facts = text(one(c, '[data-task-facts]'));
    expect(facts).toContain('paused');
    expect(facts).toContain('weekly');
    expect(facts).toContain(hhmm(resumes));
  });
});

describe('Relay · task — Run details', () => {
  it('are folded until opened', async () => {
    const { c } = await open();
    const details = one(c, 'details[data-run-details]') as HTMLDetailsElement | null;
    expect(details).not.toBeNull();
    expect(details!.open).toBe(false);
    expect(text(one(details!, 'summary'))).toContain('Run details');
  });

  it('a number nobody reported reads "not reported"; a reported zero reads 0', async () => {
    const { c } = await open();
    const detail = (key: string) => text(one(c, `[data-run-detail="${key}"]`));
    // Tokens cached and peak memory are null in the fixture; compactions is 0 (#425: rows
    // are keyed by their label).
    expect(detail('Tokens cached')).toContain('not reported');
    expect(detail('Tokens cached')).not.toMatch(/\b0\b/);
    expect(detail('Peak memory')).toContain('not reported');
    expect(detail('Compactions')).toMatch(/\b0\b/);
    expect(detail('Compactions')).not.toContain('not reported');
    expect(detail('Repository')).toContain('sensei-hq/sensei');
  });

  it('the totals say how many turns reported', async () => {
    const { c } = await open();
    expect(text(one(c, 'details[data-run-details]'))).toContain('2 of 4 reported');
  });
});

describe('Relay · task — Handed off', () => {
  it('the frozen brief, and the files it was handed', async () => {
    const { c } = await open();
    const handoff = need(one(c, '[data-handoff]'), '[data-handoff]');
    expect(text(handoff)).toContain('Move the diagrams header onto ScreenShell; behaviour unchanged.');
    expect(all(handoff, '[data-handoff-doc]').map(text)).toEqual([
      'coord/briefs/L1-diagrams.md',
      'docs/prd/modules/diagrams.md',
    ]);
  });
});

describe('Relay · task — History', () => {
  it('newest first, as the view orders it, down to Assigned', async () => {
    const { c } = await open();
    const ids = all(c, '[data-history-entry]').map((e) => e.getAttribute('data-history-entry'));
    expect(ids).toEqual([
      'act-merged',
      'act-msg-2',
      'act-review-2',
      'act-msg',
      'act-report-2',
      'act-review-1',
      'act-report-1',
      'act-assigned',
    ]);
    expect(text(one(need(entry(c, 'act-assigned'), 'history entry act-assigned'), '[data-entry-title]'))).toBe('Assigned');
  });

  it('a work turn: the summary sent with the request for review, what to look at, the flags', async () => {
    const { c } = await open();
    const first = need(entry(c, 'act-report-1'), 'history entry act-report-1');
    expect(text(one(first, '[data-entry-title]'))).toBe('Work · impl r1');
    const rework = need(entry(c, 'act-report-2'), 'history entry act-report-2');
    expect(text(one(rework, '[data-entry-title]'))).toBe('Rework · impl r2');
    expect(text(rework)).toContain('Summary sent with the request for review');
    expect(text(rework)).toContain('A failed read now shows the error notice instead of an empty header.');
    expect(text(rework)).toContain('diagram-header.svelte.ts:42');
    expect(text(rework)).toContain('One file outside my paths (view-states) needed a one-line export.');
  });

  it('a turn’s fold: files with +/−, the outside-owned flag on that file only, gates, telemetry', async () => {
    const { c } = await open();
    const more = need(one(need(entry(c, 'act-report-2'), 'history entry act-report-2'), '[data-turn-more]'), 'the turn fold');
    expect(more).not.toBeNull();
    const files = all(more, '[data-turn-file]');
    expect(files.map((f) => f.getAttribute('data-turn-file'))).toEqual([
      'app/src/lib/diagrams/DiagramHeader.svelte',
      'app/src/lib/stores/view-states.svelte.ts',
    ]);
    expect(text(files[0])).toContain('+8');
    expect(text(files[0])).toContain('−3');
    expect(one(files[0], '[data-outside-owned]')).toBeNull();
    expect(one(files[1], '[data-outside-owned]')).not.toBeNull();
    expect(text(more)).toContain('bun run check');
    expect(text(more)).toContain('exit 0');
    // tokens_cache was not reported for this turn: not a 0.
    expect(text(one(more, '[data-turn-telemetry]'))).toContain('not reported');
  });

  it('a review turn: its outcome, its summary, and its comments', async () => {
    const { c } = await open();
    const r1 = need(entry(c, 'act-review-1'), 'history entry act-review-1');
    expect(text(one(r1, '[data-entry-title]'))).toBe('Review · impl r1');
    expect(text(one(r1, '[data-entry-outcome]'))).toBe('changes requested');
    expect(text(r1)).toContain('Review summary');
    expect(text(r1)).toContain('The move is clean, but a failed read is silent.');
    const comments = all(r1, '[data-finding]');
    expect(comments).toHaveLength(1);
    expect(text(comments[0])).toContain('missing-failure-path');
    expect(text(comments[0])).toContain('diagram-header.svelte.ts:42');

    expect(text(one(need(entry(c, 'act-review-2'), 'history entry act-review-2'), '[data-entry-outcome]'))).toBe('approved');
  });

  it('the merge shows its note', async () => {
    const { c } = await open();
    const merged = need(entry(c, 'act-merged'), 'history entry act-merged');
    expect(text(one(merged, '[data-entry-title]'))).toBe('Merged');
    expect(text(merged)).toContain('Merge note');
    expect(text(merged)).toContain('Merged the two reviewed commits into develop.');
  });

  it('Message from you: to whom, and sent / delivered / read; the receipts are not entries', async () => {
    const receipts = (kinds: Array<'delivered' | 'read'>): RelayAction[] =>
      F.history().filter(
        (a) => !(a.kind === 'delivered' || a.kind === 'read') || kinds.includes(a.kind as 'delivered' | 'read'),
      );
    const cases: Array<[Array<'delivered' | 'read'>, string]> = [
      [['delivered', 'read'], 'read'],
      [['delivered'], 'delivered'],
      [[], 'sent'],
    ];
    for (const [kinds, state] of cases) {
      const { c } = await open({ task: F.ok(F.taskView({ history: receipts(kinds) })) });
      const msg = need(entry(c, 'act-msg'), 'history entry act-msg');
      expect(text(one(msg, '[data-entry-title]'))).toBe('Message from you');
      expect(text(msg)).toContain('L1');
      expect(text(msg)).toContain('Keep the old keyboard shortcut for the toolbar working.');
      expect(one(msg, '[data-message-state]')?.getAttribute('data-message-state'), kinds.join('+')).toBe(state);
      expect(entry(c, 'act-read')).toBeNull();
      expect(entry(c, 'act-delivered')).toBeNull();
    }
  });
});

describe('Relay · task — its open decision', () => {
  it('shows with Answer, and the answer goes to the daemon from the desktop', async () => {
    const view = F.taskView({ decision: F.decision({ task_id: 't-header' }) });
    const { c, api } = await open({ task: F.ok(view) });
    const row = need(one(c, '[data-decision]'), '[data-decision]');
    expect(text(row)).toContain('Where does the activity log live?');
    need(one(row, '[data-option="db"]'), '[data-option="db"]').click();
    await tick();
    need(one(row, '[data-decision-answer]'), '[data-decision-answer]').click();
    await vi.waitFor(() => expect(api.tryAnswerDecision).toHaveBeenCalledTimes(1));
    expect(api.tryAnswerDecision).toHaveBeenCalledWith({
      decision_id: 'ask-1',
      option: 'db',
      note: null,
      surface: 'desktop',
    });
  });
});

describe('Relay · task — a failed read', () => {
  const down = () => F.fail(0, 'connection refused');

  it('is the fault Notice with Retry, never an empty history', async () => {
    const { c } = await open({ task: down() });
    const notice = need(one(c, '[data-relay-error]'), '[data-relay-error]');
    expect(notice).not.toBeNull();
    expect(notice.getAttribute('data-tone')).toBe('fault');
    expect(text(notice)).toContain('connection refused');
    expect(one(notice, '[data-action="retry"]')).not.toBeNull();
    expect(one(c, '[data-history-entry]')).toBeNull();
    expect(one(c, '[data-run-details]')).toBeNull();
  });

  it('Retry reads this task once, and recovers', async () => {
    const { c, api } = await open({ task: down(), taskId: 't-billing' });
    api.tryGetTask.mockClear();
    api.tryGetTask.mockResolvedValue(F.ok(F.taskView()));
    need(one(c, '[data-relay-error] [data-action="retry"]'), '[data-relay-error] [data-action="retry"]').click();
    await vi.waitFor(() => expect(one(c, '[data-history-entry]')).not.toBeNull());
    expect(one(c, '[data-relay-error]')).toBeNull();
    expect(api.tryGetTask).toHaveBeenCalledTimes(1);
    expect(api.tryGetTask).toHaveBeenCalledWith(F.CREW_ID, 't-billing');
  });

  it('a Retry that fails again shows the new reason', async () => {
    const { c, api } = await open({ task: down() });
    api.tryGetTask.mockResolvedValue(F.fail(502, 'Bad Gateway'));
    need(one(c, '[data-relay-error] [data-action="retry"]'), '[data-relay-error] [data-action="retry"]').click();
    await vi.waitFor(() => expect(text(one(c, '[data-relay-error]'))).toContain('Bad Gateway'));
    expect(text(one(c, '[data-relay-error]'))).not.toContain('connection refused');
    expect(one(c, '[data-history-entry]')).toBeNull();
  });

  it('two presses of Retry are one read', async () => {
    const { c, api } = await open({ task: down() });
    const pending = F.deferred<ApiResult<RelayTaskView>>();
    api.tryGetTask.mockClear();
    api.tryGetTask.mockReturnValue(pending.promise);
    const retry = need(one(c, '[data-relay-error] [data-action="retry"]'), 'the Retry on the error notice');
    retry.click();
    retry.click();
    pending.resolve(F.ok(F.taskView()));
    await vi.waitFor(() => expect(one(c, '[data-history-entry]')).not.toBeNull());
    expect(api.tryGetTask).toHaveBeenCalledTimes(1);
  });
});

// ─── r2 (review relay-screen-tests-r1) ──────────────────────────────────────

const hhmm = (iso: string) => {
  const d = new Date(iso);
  return `${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`;
};

describe('Relay · task r2 — the shell (F1)', () => {
  it('search is off on the task view too', async () => {
    const { c } = await open();
    expect(vi.mocked(ScreenShell)).toHaveBeenCalled();
    expect((vi.mocked(ScreenShell).mock.calls[0][1] as unknown as Record<string, unknown>).search).toBe(false);
    expect(one(c, '[data-screen-search]')).toBeNull();
  });
});

describe('Relay · task r2 — scoped by the crew and the user (F3, repository ruling)', () => {
  // The daemon reads (user, crew, task): t-header is in CREW_ID; any other crew, or another
  // user's task id, is a 404. The screen shows "not found", never a row.
  const perCrew = (api: F.FakeRelayApi) =>
    api.tryGetTask.mockImplementation(async (crewId: string, taskId: string) =>
      crewId === F.CREW_ID && taskId === 't-header' ? F.ok(F.taskView()) : F.fail(404, 'task not found'),
    );

  it('a task from another crew, or another user’s, is "not found": no header, history or Retry', async () => {
    const cases: Array<[string, string, string]> = [
      ['another crew', F.OTHER_CREW_ID, 't-header'],
      ['another user', F.CREW_ID, 't-someone-elses'],
    ];
    for (const [why, crewId, taskId] of cases) {
      const { c, api } = await open({ workspace: 'w2', crewId, taskId, setup: perCrew });
      expect(api.tryGetTask, why).toHaveBeenCalledWith(crewId, taskId);
      const notFound = need(one(c, '[data-relay-not-found]'), `the not-found notice (${why})`);
      expect(notFound.getAttribute('data-tone'), why).toBe('refusal');
      expect(one(notFound, '[data-action="retry"]'), why).toBeNull();
      expect(one(c, 'a[data-back]')?.getAttribute('href'), why).toBe('/workspace/w2/relay');
      expect(one(c, '[data-task-title]'), why).toBeNull();
      expect(one(c, '[data-history-entry]'), why).toBeNull();
      expect(one(c, '[data-run-details]'), why).toBeNull();
      expect(one(c, '[data-relay-error]'), why).toBeNull();
      expect(text(c), why).not.toContain('Diagrams header on ScreenShell');
    }
  });

  it('the same task under its own crew draws', async () => {
    const { c, api } = await open({ crewId: F.CREW_ID, taskId: 't-header', setup: perCrew });
    expect(api.tryGetTask).toHaveBeenCalledWith(F.CREW_ID, 't-header');
    expect(text(one(c, '[data-task-title]'))).toBe('Diagrams header on ScreenShell');
    expect(one(c, '[data-relay-not-found]')).toBeNull();
  });
});

describe('Relay · task r2 — its decision (F4, L3)', () => {
  const withDecision = () => F.ok(F.taskView({ decision: F.decision({ task_id: 't-header' }) }));

  it('two presses of Answer answer once', async () => {
    const { c, api } = await open({ task: withDecision() });
    const pending = F.deferred<ApiResult<{ action_id: string }>>();
    api.tryAnswerDecision.mockReturnValue(pending.promise);
    const row = need(one(c, '[data-decision]'), 'the decision');
    need(one(row, '[data-option="db"]'), 'the db option').click();
    await tick();
    const answer = need(one(row, '[data-decision-answer]'), 'Answer');
    answer.click();
    answer.click();
    pending.resolve(F.ok({ action_id: 'act-answer' }));
    await vi.waitFor(() => expect(api.tryAnswerDecision).toHaveBeenCalled());
    await tick();
    expect(api.tryAnswerDecision).toHaveBeenCalledTimes(1);
  });

  it('an answer re-reads the task, and the decision leaves', async () => {
    const { c, api } = await open({ task: withDecision() });
    api.tryGetTask.mockResolvedValue(F.ok(F.taskView({ decision: null })));
    const row = need(one(c, '[data-decision]'), 'the decision');
    need(one(row, '[data-option="file"]'), 'the file option').click();
    await tick();
    need(one(row, '[data-decision-answer]'), 'Answer').click();
    await vi.waitFor(() => expect(one(c, '[data-decision]')).toBeNull());
    expect(api.tryGetTask).toHaveBeenCalledTimes(2);
    expect(api.tryGetTask).toHaveBeenLastCalledWith(F.CREW_ID, 't-header');
  });

  it('a refused answer says why and keeps the decision', async () => {
    const { c, api } = await open({ task: withDecision() });
    api.tryAnswerDecision.mockResolvedValue(F.fail(409, 'already answered on the dōjō'));
    const row = need(one(c, '[data-decision]'), 'the decision');
    need(one(row, '[data-option="db"]'), 'the db option').click();
    await tick();
    need(one(row, '[data-decision-answer]'), 'Answer').click();
    await vi.waitFor(() => expect(text(one(c, '[data-decision-error]'))).toContain('already answered on the dōjō'));
    expect(one(c, '[data-decision]')).not.toBeNull();
  });
});

describe('Relay · task follow-ups — a re-read that fails, and a re-offer with no lane', () => {
  const withDecision = () => F.ok(F.taskView({ decision: F.decision({ task_id: 't-header' }) }));

  // N8: the write landed, the re-read did not. Drawing the view we kept would pass
  // off what the daemon said before the answer as what it says now (D83).
  it('an answer whose re-read fails is the fault Notice, and a Retry brings the task back', async () => {
    const { c, api } = await open({ task: withDecision() });
    api.tryGetTask.mockResolvedValueOnce(F.fail(502, 'the daemon dropped the read'));

    const row = need(one(c, '[data-decision]'), 'the decision');
    need(one(row, '[data-option="file"]'), 'the file option').click();
    await tick();
    need(one(row, '[data-decision-answer]'), 'Answer').click();

    await vi.waitFor(() => expect(one(c, '[data-relay-error]')).not.toBeNull());
    const notice = need(one(c, '[data-relay-error]'), 'the fault Notice after a failed re-read');
    expect(text(notice)).toContain('the daemon dropped the read');
    // The stale view is not drawn as if it were fresh.
    expect(one(c, '[data-history-entry]')).toBeNull();
    expect(one(c, '[data-task-title]')).toBeNull();

    api.tryGetTask.mockResolvedValue(F.ok(F.taskView({ decision: null })));
    need(one(notice, '[data-action="retry"]'), 'the Notice’s Retry').click();
    await vi.waitFor(() => expect(one(c, '[data-history-entry]')).not.toBeNull());
    expect(one(c, '[data-relay-error]')).toBeNull();
    expect(api.tryGetTask).toHaveBeenCalledTimes(3);
  });

  // N10: a lane that hands a task back to the backlog has no lane to name.
  describe('a re-offer', () => {
    const view = (to_lane: string | null) =>
      F.ok(F.taskView({ history: [F.action('reoffered', { to_lane }, { id: 'act-reoffered', lane_code: 'L0' })] }));
    const title = (c: HTMLElement) => text(one(need(entry(c, 'act-reoffered'), 'the re-offer step'), '[data-entry-title]'));

    it('to a lane names it', async () => {
      const { c } = await open({ task: view(F.LANE.L3) });
      expect(title(c)).toBe('Re-offered to L3');
    });

    it('with no lane says it went back to the queue, never "Re-offered to —"', async () => {
      const { c } = await open({ task: view(null) });
      expect(title(c)).toBe('Returned to the queue');
    });
  });
});

describe('Relay · task r2 — receipts fold per message (F5)', () => {
  it('each message carries its own receipts: one read, one only delivered', async () => {
    const { c } = await open();
    const first = need(one(c, '[data-history-entry="act-msg"]'), 'history entry act-msg');
    const second = need(one(c, '[data-history-entry="act-msg-2"]'), 'history entry act-msg-2');
    expect(one(first, '[data-message-state]')?.getAttribute('data-message-state')).toBe('read');
    expect(one(second, '[data-message-state]')?.getAttribute('data-message-state')).toBe('delivered');
    for (const receipt of ['act-read', 'act-delivered', 'act-delivered-2']) {
      expect(one(c, `[data-history-entry="${receipt}"]`), receipt).toBeNull();
    }
    // `to` is stored as the lane's uuid; the reader sees its code.
    expect(text(second)).toContain('L1');
    expect(text(c)).not.toContain(F.LANE.L1);
  });
});

describe('Relay · task r2 — the steps beyond the turns (F8)', () => {
  const stepsView = () =>
    F.taskView({
      history: F.historyWithSteps(),
      handoff: { body_md: 'Also keep the old keyboard shortcut working.', docs: ['coord/briefs/L1-diagrams-r2.md'], at: F.minutesAgo(120) },
    });

  it('a brief update is its own step, and Handed off shows the latest brief', async () => {
    const { c } = await open({ task: F.ok(stepsView()) });
    const updated = need(one(c, '[data-history-entry="act-brief-2"]'), 'history entry act-brief-2');
    expect(text(one(updated, '[data-entry-title]'))).toBe('Brief updated');
    expect(text(updated)).toContain('Also keep the old keyboard shortcut working.');
    const handoff = need(one(c, '[data-handoff]'), 'Handed off');
    expect(text(handoff)).toContain('Also keep the old keyboard shortcut working.');
    expect(text(handoff)).not.toContain('Move the diagrams header onto ScreenShell');
  });

  it('a lost lease and its re-offer: "Lease lost", then "Re-offered to L3"', async () => {
    const { c } = await open({ task: F.ok(stepsView()) });
    expect(text(one(need(one(c, '[data-history-entry="act-interrupted"]'), 'the lease-lost step'), '[data-entry-title]'))).toBe('Lease lost');
    const reoffered = need(one(c, '[data-history-entry="act-reoffered"]'), 'the re-offer step');
    expect(text(one(reoffered, '[data-entry-title]'))).toBe('Re-offered to L3');
    expect(text(c)).not.toContain(F.LANE.L3);
  });

  it('started, asked, answered and compacted are plain one-line steps, never blank', async () => {
    const { c } = await open({ task: F.ok(stepsView()) });
    for (const id of ['act-started', 'act-asked', 'act-answered', 'act-compacted']) {
      const step = need(one(c, `[data-history-entry="${id}"]`), `history entry ${id}`);
      const title = text(one(step, '[data-entry-title]'));
      expect(title, id).not.toBe('');
      expect(text(step), id).not.toMatch(/undefined|null|\[object/);
      expect(one(step, '[data-turn-more]'), id).toBeNull();
    }
    expect(all(c, '[data-history-entry]').map((e) => e.getAttribute('data-history-entry'))).toEqual([
      'act-reoffered',
      'act-interrupted',
      'act-compacted',
      'act-answered',
      'act-asked',
      'act-brief-2',
      'act-started',
      'act-assigned',
    ]);
  });
});

describe('Relay · task r2 — the facts line (L2)', () => {
  it('names the member and the time, assigned → completed', async () => {
    const view = F.taskView();
    const { c } = await open({ task: F.ok(view) });
    const facts = text(one(c, '[data-task-facts]'));
    expect(facts).toMatch(/claude-code|Claude Code/);
    expect(facts).toContain(hhmm(view.task.assigned_at!));
    expect(facts).toContain(hhmm(view.task.completed_at!));
  });
});

describe('Relay · task impl r2 — who a row is from (L4 F2)', () => {
  // Q12 puts lanes in this history: a lane sends messages and asks other lanes.
  // A lane's message titled "Message from you" gives the person its words.
  const laneSent = () =>
    F.action(
      'message',
      { to: { kind: 'person' }, surface: 'lane' },
      { id: 'act-msg-lane', lane_code: 'L3', body_md: 'Sync is refusing the repository.', created_at: F.minutesAgo(15) },
    );

  it('a lane’s message names the lane; the person’s own still reads yours', async () => {
    const { c } = await open({
      task: F.ok(F.taskView({ history: [laneSent(), ...F.history().filter((a) => a.id === 'act-msg')] })),
    });
    const mine = need(entry(c, 'act-msg'), 'history entry act-msg');
    expect(text(one(mine, '[data-entry-title]'))).toBe('Message from you');

    const theirs = need(entry(c, 'act-msg-lane'), 'history entry act-msg-lane');
    expect(text(theirs)).toContain('Sync is refusing the repository.');
    expect(text(one(theirs, '[data-entry-title]'))).toBe('Message from L3');
    expect(text(theirs)).not.toContain('from you');
  });

  it('an ask directed at a lane names that lane; one at the person reads "Asked you"', async () => {
    const ask = (asked_of: string, id: string, question: string) =>
      F.action(
        'asked',
        { context: null, options: [], recommended: null, blocks: [], asked_of },
        { id, lane_code: 'L0', body_md: question, created_at: F.minutesAgo(60) },
      );
    const { c } = await open({
      task: F.ok(
        F.taskView({
          history: [
            ask(F.LANE.L1, 'act-ask-lane', 'Is the billing export ready?'),
            ask('person', 'act-ask-person', 'Where does the activity log live?'),
          ],
        }),
      ),
    });
    expect(text(one(need(entry(c, 'act-ask-person'), 'the ask of the person'), '[data-entry-title]'))).toBe('Asked you');
    const atLane = need(entry(c, 'act-ask-lane'), 'the ask of a lane');
    expect(text(atLane)).toContain('Is the billing export ready?');
    expect(text(one(atLane, '[data-entry-title]'))).toBe('Asked L1');
    // A uuid never reaches the screen.
    expect(text(c)).not.toContain(F.LANE.L1);
  });
});
