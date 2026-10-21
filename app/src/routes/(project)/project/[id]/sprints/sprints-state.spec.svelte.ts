// The Sprints screen's state (docs/spec/screen/project-sprints.md).
//
// What must never happen here: "no tracker connected" because a read failed, a
// 0 where the daemon said null, or a sprint total built by adding its tickets.
import { describe, it, expect } from 'vitest';
import { SprintsState, usageCells, type SprintsApi } from './sprints-state.svelte.js';
import type { ApiResult } from '$lib/api.js';
import type {
  SprintUsageRow,
  TrackerConnection,
  TrackerTaskDetail,
  TrackerTaskRow,
  TrackerUsage,
  UnlinkedSession,
} from '$lib/types.js';

const P = 'p-1';

const usage = (over: Partial<TrackerUsage> = {}): TrackerUsage => ({
  sessions: 0,
  by_source: { branch: 0, prompt: 0 },
  shared: 0,
  active_secs: null,
  active_n: 0,
  tokens_fresh: null,
  tokens_fresh_n: 0,
  cache_read: null,
  cache_read_n: 0,
  cache_write: null,
  cache_write_n: 0,
  tokens_out: null,
  tokens_out_n: 0,
  tokens_reasoning: null,
  tokens_reasoning_n: 0,
  metered_cost: null,
  metered_cost_n: 0,
  ...over,
});

const sprint = (over: Partial<SprintUsageRow> = {}): SprintUsageRow => ({
  id: 's12',
  name: 'Sprint 12',
  state: 'closed',
  start_at: '2026-06-15T12:00:00Z',
  end_at: '2026-06-25T12:00:00Z',
  complete_at: null,
  tasks: 5,
  candidates_in_window: 3,
  linked_sprint: usage({ sessions: 2, by_source: { branch: 2, prompt: 0 }, active_secs: 3840, active_n: 2 }),
  linked_other: usage({ sessions: 1 }),
  unlinked: usage(),
  reason: null,
  ...over,
});

const task = (over: Partial<TrackerTaskRow> = {}): TrackerTaskRow => ({
  key: 'AUTH-2',
  summary: 'Login',
  issue_type: 'Story',
  status: 'To Do',
  current_sprint: { id: 's12', name: 'Sprint 12' },
  sprints: [{ id: 's12', name: 'Sprint 12' }],
  removed: false,
  usage: usage({ sessions: 3, by_source: { branch: 3, prompt: 1 }, active_secs: 3840, active_n: 2, tokens_fresh: 7700, tokens_fresh_n: 2 }),
  ...over,
});

const tracker: TrackerConnection = {
  id: 't-1',
  kind: 'jira',
  base_url: 'https://acme.atlassian.net',
  account: 'dev@example.com',
  external_scope: 'AUTH',
  board_ref: '42',
  board_name: 'AUTH Scrum',
  status: 'connected',
  last_success_at: '2026-10-18T11:58:00Z',
  last_error: null,
  counts: { tasks: 3, sprints: 2 },
};

const ok = <T,>(data: T): ApiResult<T> => ({ ok: true, data });
const fail = (status: number, code?: string) =>
  ({
    ok: false,
    error: code ? { status, message: code, body: { error: code } } : { status, message: 'refused' },
  }) as const;

function daemon(over: Partial<SprintsApi> = {}) {
  const calls: string[] = [];
  const api: SprintsApi = {
    tryListTrackers: async () => ok([tracker]),
    tryListSprints: async () => {
      calls.push('sprints');
      return ok([sprint({ id: 's13', name: 'Sprint 13', state: 'active' }), sprint()]);
    },
    tryListTrackerTasks: async (_p, s) => {
      calls.push(`tasks:${s}`);
      return ok([task()]);
    },
    tryGetTrackerTask: async (_p, key) => {
      calls.push(`task:${key}`);
      return ok<TrackerTaskDetail>({
        task: { key: 'IAM-5', summary: 'Login', issue_type: 'Story', status: 'Done', removed: false },
        usage: usage({ sessions: 2 }),
        by_repo: [{ repository_id: 'r1', name: 'backend', usage: usage({ sessions: 2 }) }],
        sessions: [
          {
            id: 'sess-1',
            started_at: '2026-06-16T09:00:00Z',
            links: [
              { source: 'branch', matched_key: 'AUTH-420', mentions: 1 },
              { source: 'prompt', matched_key: 'IAM-5', mentions: 2 },
            ],
          },
        ],
      });
    },
    tryListUnlinkedSessions: async (_p, s) => {
      calls.push(`unlinked:${s}`);
      return ok<UnlinkedSession[]>([{ id: 'u1', started_at: '2026-06-17T09:00:00Z', task: 'refactor' }]);
    },
    ...over,
  };
  return { api, calls };
}

async function ready(over: Partial<SprintsApi> = {}) {
  const d = daemon(over);
  const s = new SprintsState(d.api, P);
  await s.load();
  return { s, ...d };
}

describe('loading', () => {
  it('selects the active sprint and reads its tickets', async () => {
    const { s, calls } = await ready();
    expect(s.view).toBe('ready');
    expect(s.boardTitle).toBe('AUTH Scrum');
    expect(s.selected?.name).toBe('Sprint 13');
    expect(calls).toContain('tasks:s13');
    expect(s.tasks?.map((t) => t.key)).toEqual(['AUTH-2']);
  });

  it('without an active sprint, selects the newest', async () => {
    const { s } = await ready({
      tryListSprints: async () => ok([sprint({ id: 'a' }), sprint({ id: 'b', name: 'Sprint 11' })]),
    });
    expect(s.selected?.id).toBe('a');
  });

  it('no tracker is the not-connected state', async () => {
    const { s } = await ready({ tryListSprints: async () => fail(404, 'not_connected') });
    expect(s.view).toBe('not_connected');
  });

  // A failed read is never "no tracker connected".
  it('a failed read is the daemon error, in the same words as Integrations', async () => {
    const down = await ready({ tryListSprints: async () => fail(0) });
    expect([down.s.view, down.s.loadError]).toEqual(['daemon_error', 'The sensei daemon didn’t respond.']);
    const broken = await ready({ tryListSprints: async () => fail(500, 'internal') });
    expect([broken.s.view, broken.s.loadError]).toEqual([
      'daemon_error',
      'The sensei daemon hit a database error.',
    ]);
  });

  it('a failed ticket read says so instead of showing an empty sprint', async () => {
    const { s } = await ready({ tryListTrackerTasks: async () => fail(500, 'internal') });
    expect(s.tasks).toBe(null);
    expect(s.tasksError).toBe('The sensei daemon hit a database error.');
  });
});

describe('selecting', () => {
  it('reads the chosen sprint’s tickets; a slower earlier answer never wins', async () => {
    let releaseFirst!: () => void;
    const { s, api } = await ready();
    api.tryListTrackerTasks = (_p, id) =>
      id === 's13'
        ? new Promise((r) => {
            releaseFirst = () => r(ok([task({ key: 'STALE-1' })]));
          })
        : Promise.resolve(ok([task({ key: 'AUTH-9' })]));
    const first = s.select('s13');
    const second = s.select('s12');
    await second;
    releaseFirst();
    await first;
    expect(s.selected?.id).toBe('s12');
    expect(s.tasks?.map((t) => t.key)).toEqual(['AUTH-9']);
  });
});

describe('the usage strip', () => {
  it('shows the window total and three parts, from the sprint’s own numbers', async () => {
    const { s } = await ready();
    await s.select('s12');
    expect(s.strip).toEqual({
      kind: 'window',
      total: '3 sessions in this sprint’s window',
      parts: [
        { label: 'Linked to this sprint', sessions: '2', active: '1h 04m', fresh: '—' },
        { label: 'Linked to other tickets', sessions: '1', active: '—', fresh: '—' },
        { label: 'Not linked', sessions: '0', active: '—', fresh: '—' },
      ],
    });
  });

  it('a sprint without a window says why, with no zeros', async () => {
    const { s } = await ready({
      tryListSprints: async () =>
        ok([
          sprint({ id: 'f', state: 'future', reason: 'not_started', candidates_in_window: null, linked_sprint: null, linked_other: null, unlinked: null }),
          sprint({ id: 'n', reason: 'no_dates', candidates_in_window: null, linked_sprint: null, linked_other: null, unlinked: null }),
        ]),
    });
    await s.select('f');
    expect(s.strip).toEqual({ kind: 'reason', text: 'Not started. Usage counts from the sprint’s start.' });
    await s.select('n');
    expect(s.strip).toEqual({
      kind: 'reason',
      text: 'Jira has no dates for this sprint, so its sessions can’t be placed in it.',
    });
  });

  it('lists a sprint’s rows with state, dates and ticket count', async () => {
    const { s } = await ready();
    expect(s.rows[1]).toEqual({ id: 's12', name: 'Sprint 12', state: 'closed', dates: 'Jun 15 – Jun 25', tickets: '5 tickets', current: false });
    expect(s.rows[0].current).toBe(true);
  });
});

describe('ticket cells', () => {
  it('formats known values and leaves unknown ones as —', () => {
    expect(usageCells(task().usage)).toEqual({
      sessions: '3 (3/1)',
      active: '1h 04m',
      fresh: '7.7k',
      out: '—',
      cost: '—',
    });
    expect(usageCells(usage({ metered_cost: 1.5, metered_cost_n: 1 })).cost).toBe('$1.50');
    expect(usageCells(usage()).sessions).toBe('0 (0/0)');
  });

  it('marks a ticket that has moved on, and notes shared sessions', async () => {
    const { s } = await ready({
      tryListTrackerTasks: async () =>
        ok([
          task({ current_sprint: { id: 's14', name: 'Sprint 14' } }),
          task({
            key: 'AUTH-3',
            current_sprint: { id: 's13', name: 'Sprint 13' },
            usage: usage({ sessions: 1, shared: 1 }),
          }),
        ]),
    });
    expect(s.movedTo(s.tasks![0])).toBe('now in Sprint 14');
    expect(s.movedTo(s.tasks![1])).toBe(null);
    expect(s.sharedNote).toBe(
      'A session linked to several tickets counts in full for each; the sprint total counts it once.',
    );
  });
});

describe('a ticket', () => {
  it('opens to its repos and sessions; an old key shows as written', async () => {
    const { s, calls } = await ready();
    await s.openTicket('AUTH-420');
    expect(calls).toContain('task:AUTH-420');
    expect(s.ticket?.task.key).toBe('IAM-5');
    expect(s.linksText(s.ticket!.sessions[0])).toBe('branch · prompt ×2 · as AUTH-420');
    s.closeTicket();
    expect(s.ticket).toBe(null);
  });

  it('a failed ticket read says so', async () => {
    const { s } = await ready({ tryGetTrackerTask: async () => fail(404, 'task_not_found') });
    await s.openTicket('AUTH-1');
    expect(s.ticket).toBe(null);
    expect(s.ticketError).toBe('No ticket with that key in sensei.');
  });
});

describe('unlinked sessions', () => {
  it('reads them for the selected sprint on demand', async () => {
    const { s, calls } = await ready();
    await s.toggleUnlinked();
    expect(calls).toContain('unlinked:s13');
    expect(s.unlinked?.map((u) => u.id)).toEqual(['u1']);
    await s.toggleUnlinked();
    expect(s.unlinked).toBe(null);
  });

  it('a failed read is an error, not an empty list', async () => {
    const { s } = await ready({ tryListUnlinkedSessions: async () => fail(0) });
    await s.toggleUnlinked();
    expect(s.unlinked).toBe(null);
    expect(s.unlinkedError).toBe('The sensei daemon didn’t respond.');
  });
});
