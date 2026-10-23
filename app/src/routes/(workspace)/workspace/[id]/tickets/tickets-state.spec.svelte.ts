// The Sprints screen's state (docs/spec/screen/workspace-tickets.md).
//
// What must never happen here: "no tracker connected" because a read failed, a
// 0 where the daemon said null, or a sprint total built by adding its tickets.
import { describe, it, expect } from 'vitest';
import { ALL, BACKLOG, TicketsState, usageCells, type SprintsApi } from './tickets-state.svelte.js';
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
  api_cost: null,
  api_cost_n: 0,
  api_cost_partial: 0,
  claude_secs: null,
  claude_n: 0,
  user_secs: null,
  user_n: 0,
  wall_secs: null,
  wall_n: 0,
  first_at: null,
  last_at: null,
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
  level: 'sprint',
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
    tryListSprints: async (_p, t) => {
      calls.push(`sprints:${t}`);
      return ok([sprint({ id: 's13', name: 'Sprint 13', state: 'active' }), sprint()]);
    },
    tryListTrackerTasks: async (_p, t, s) => {
      calls.push(`tasks:${t}/${s}`);
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
    tryGetBacklog: async (_p, t) => {
      calls.push(`backlog:${t}`);
      return ok({ tickets: 2, usage: usage({ sessions: 1, by_source: { branch: 1, prompt: 0 }, active_secs: 600, active_n: 1 }) });
    },
    tryListBacklogTasks: async (_p, t) => {
      calls.push(`backlog-tasks:${t}`);
      return ok([task({ key: 'P-783', current_sprint: null, sprints: [] })]);
    },
    tryListUnlinkedSessions: async (_p, t, s) => {
      calls.push(`unlinked:${t}/${s}`);
      return ok<UnlinkedSession[]>([{ id: 'u1', started_at: '2026-06-17T09:00:00Z', task: 'refactor' }]);
    },
    ...over,
  };
  return { api, calls };
}

async function ready(over: Partial<SprintsApi> = {}) {
  const d = daemon(over);
  const s = new TicketsState(d.api, P);
  await s.load();
  // Most tests have one sprint board: its lane is `l`.
  return { s, l: s.lanes[0], ...d };
}

describe('loading', () => {
  it('starts with every ticket, not a sprint', async () => {
    const { s, l, calls } = await ready();
    expect(s.view).toBe('ready');
    expect(s.boardTitle).toBe('AUTH Scrum');
    expect(l.filter).toBe(ALL);
    expect(calls).toContain('tasks:t-1/undefined');
    expect(calls).not.toContain('tasks:t-1/s13');
    expect(l.tasks?.map((t) => t.key)).toEqual(['AUTH-2']);
    expect(l.strip).toBe(null);
  });

  it('no sprint board is the not-connected state, an epic board alone included', async () => {
    const none = await ready({ tryListTrackers: async () => ok([]) });
    expect(none.s.view).toBe('not_connected');
    const epicOnly = await ready({ tryListTrackers: async () => ok([{ ...tracker, level: 'epic' as const }]) });
    expect(epicOnly.s.view).toBe('not_connected');
    expect(epicOnly.calls.some((c) => c.startsWith('sprints:'))).toBe(false);
  });

  it('two sprint boards are two lanes side by side, each with its own tickets and filter (D40)', async () => {
    const mobile = { ...tracker, id: 't-2', board_ref: '44', board_name: 'Mobile Scrum' };
    const epic = { ...tracker, id: 't-3', level: 'epic' as const, board_ref: '43', board_name: 'Portfolio' };
    const { s, calls } = await ready({
      tryListTrackers: async () => ok([epic, tracker, mobile]),
      tryListSprints: async (_p, t) =>
        t === 't-2'
          ? ok([sprint({ id: 'm7', name: 'Mobile 7', state: 'active' })])
          : ok([sprint({ id: 's13', name: 'Sprint 13', state: 'active' }), sprint()]),
    });
    expect(s.lanes.map((l) => [l.id, l.title, l.filter])).toEqual([
      ['t-1', 'AUTH Scrum', ALL],
      ['t-2', 'Mobile Scrum', ALL],
    ]);
    expect(s.boardTitle).toBe('');
    expect(calls).toEqual(
      expect.arrayContaining(['tasks:t-1/undefined', 'tasks:t-2/undefined', 'backlog:t-1', 'backlog:t-2']),
    );
    expect(s.lanes[1].filterItems.map((i) => i.label)).toEqual(['All tickets', 'Mobile 7 · active', 'Backlog · 2 tickets']);

    await s.lanes[1].setFilter('m7');
    expect(calls).toContain('tasks:t-2/m7');
    expect(s.lanes[0].filter).toBe(ALL);
    await s.lanes[1].toggleUnlinked();
    expect(calls).toContain('unlinked:t-2/m7');

    await s.openTicket(s.lanes[1], 'AUTH-2');
    expect(s.backLabel).toBe('Back to Mobile Scrum · Mobile 7');
  });

  it('one lane failing to read is the daemon error, never a missing lane', async () => {
    const mobile = { ...tracker, id: 't-2', board_ref: '44' };
    const { s } = await ready({
      tryListTrackers: async () => ok([tracker, mobile]),
      tryListSprints: async (_p, t) => (t === 't-2' ? fail(500, 'internal') : ok([sprint()])),
    });
    expect([s.view, s.loadError]).toEqual(['daemon_error', 'The sensei daemon hit a database error.']);
  });

  // A failed read is never "no tracker connected".
  it('a failed read is the daemon error, in the same words as Integrations', async () => {
    const down = await ready({ tryListTrackers: async () => fail(0) });
    expect([down.s.view, down.s.loadError]).toEqual(['daemon_error', 'The sensei daemon didn’t respond.']);
    const broken = await ready({ tryListSprints: async () => fail(500, 'internal') });
    expect([broken.s.view, broken.s.loadError]).toEqual([
      'daemon_error',
      'The sensei daemon hit a database error.',
    ]);
  });

  it('a failed ticket read says so instead of showing no tickets', async () => {
    const { s, l } = await ready({ tryListTrackerTasks: async () => fail(500, 'internal') });
    expect(l.tasks).toBe(null);
    expect(l.tasksError).toBe('The sensei daemon hit a database error.');
  });

  // A failed backlog read is never "Backlog · 0 tickets".
  it('a failed backlog read is the daemon error', async () => {
    const { s, l } = await ready({ tryGetBacklog: async () => fail(500, 'internal') });
    expect([s.view, s.loadError]).toEqual(['daemon_error', 'The sensei daemon hit a database error.']);
  });
});

describe('the sprint filter', () => {
  it('offers All tickets, each sprint newest first, then Backlog', async () => {
    const { s, l } = await ready();
    expect(l.filterItems).toEqual([
      { value: ALL, label: 'All tickets' },
      { value: 's13', label: 'Sprint 13 · active' },
      { value: 's12', label: 'Sprint 12 · closed' },
      { value: BACKLOG, label: 'Backlog · 2 tickets' },
    ]);
  });

  it('a sprint reads its tickets and shows its own usage', async () => {
    const { s, l, calls } = await ready();
    await l.setFilter('s12');
    expect(calls).toContain('tasks:t-1/s12');
    expect(l.filterLabel).toBe('Sprint 12');
    expect(l.strip).toEqual({
      kind: 'window',
      total: '3 sessions in this sprint’s window',
      parts: [
        { label: 'Linked to this sprint', sessions: '2', active: '1h 04m', fresh: '—' },
        { label: 'Linked to other tickets', sessions: '1', active: '—', fresh: '—' },
        { label: 'Not linked', sessions: '0', active: '—', fresh: '—' },
      ],
    });
  });

  it('Backlog reads the tickets in no sprint and shows their total, each session once', async () => {
    const { s, l, calls } = await ready();
    await l.setFilter(BACKLOG);
    expect(calls).toContain('backlog-tasks:t-1');
    expect(l.tasks?.map((t) => t.key)).toEqual(['P-783']);
    expect(l.filterLabel).toBe('Backlog');
    expect(l.strip).toEqual({
      kind: 'window',
      total: '2 tickets in no sprint',
      parts: [{ label: 'Linked to backlog tickets', sessions: '1', active: '10m', fresh: '—' }],
    });
  });

  it('back to All tickets drops the strip', async () => {
    const { s, l } = await ready();
    await l.setFilter('s12');
    await l.setFilter(ALL);
    expect([l.filter, l.strip]).toEqual([ALL, null]);
  });

  it('a slower earlier answer never wins', async () => {
    let releaseFirst!: () => void;
    const { s, l, api } = await ready();
    api.tryListTrackerTasks = (_p, _t, id) =>
      id === 's13'
        ? new Promise((r) => {
            releaseFirst = () => r(ok([task({ key: 'STALE-1' })]));
          })
        : Promise.resolve(ok([task({ key: 'AUTH-9' })]));
    const first = l.setFilter('s13');
    const second = l.setFilter('s12');
    await second;
    releaseFirst();
    await first;
    expect(l.filter).toBe('s12');
    expect(l.tasks?.map((t) => t.key)).toEqual(['AUTH-9']);
  });

  it('a sprint without a window says why, with no zeros', async () => {
    const { s, l } = await ready({
      tryListSprints: async () =>
        ok([
          sprint({ id: 'f', state: 'future', reason: 'not_started', candidates_in_window: null, linked_sprint: null, linked_other: null, unlinked: null }),
          sprint({ id: 'n', reason: 'no_dates', candidates_in_window: null, linked_sprint: null, linked_other: null, unlinked: null }),
        ]),
    });
    await l.setFilter('f');
    expect(l.strip).toEqual({ kind: 'reason', text: 'Not started. Usage counts from the sprint’s start.' });
    await l.setFilter('n');
    expect(l.strip).toEqual({
      kind: 'reason',
      text: 'Jira has no dates for this sprint, so its sessions can’t be placed in it.',
    });
  });
});

describe('ticket rows', () => {
  it('formats known values and leaves unknown ones as —', () => {
    expect(usageCells(task().usage)).toEqual({
      sessions: '3 (3/1)',
      active: '1h 04m',
      fresh: '7.7k',
      out: '—',
      cost: '—',
      claude: '—',
      you: '—',
      wall: '—',
      cycle: '—',
    });
    // The cost cell is the API cost (session API cost C5), not a source's own figure.
    expect(usageCells(usage({ metered_cost: 1.5, metered_cost_n: 1 })).cost).toBe('—');
    expect(usageCells(usage({ api_cost: 1.2449, api_cost_n: 2 })).cost).toBe('$1.24');
    expect(usageCells(usage({ api_cost: 1.2449, api_cost_n: 2, api_cost_partial: 1 })).cost).toBe('$1.24+');
    expect(usageCells(usage({ api_cost: 0.004, api_cost_n: 1 })).cost).toBe('<$0.01');
    expect(usageCells(usage()).sessions).toBe('0 (0/0)');
  });

  it('splits active AI time into Claude and you, and spans the cycle from first start to last end', () => {
    const c = usageCells(
      usage({
        active_secs: 2400,
        active_n: 2,
        claude_secs: 1800,
        claude_n: 2,
        user_secs: 600,
        user_n: 2,
        wall_secs: 3000,
        wall_n: 2,
        first_at: '2026-10-05T10:00:00Z',
        last_at: '2026-10-07T17:20:00Z',
      }),
    );
    expect([c.active, c.claude, c.you, c.wall, c.cycle]).toEqual(['40m', '30m', '10m', '50m', '55h 20m']);
  });

  it('has no cycle time while no session has ended', () => {
    expect(usageCells(usage({ first_at: '2026-10-05T10:00:00Z', last_at: null })).cycle).toBe('—');
  });

  it('name each ticket’s current sprint, or Backlog', async () => {
    const { s, l } = await ready();
    expect(l.sprintOf(task({ current_sprint: { id: 's14', name: 'Sprint 14' } }))).toBe('Sprint 14');
    expect(l.sprintOf(task({ current_sprint: null, sprints: [] }))).toBe('Backlog');
  });

  it('can narrow to the tickets with AI sessions, and says how many are shown', async () => {
    const { s, l } = await ready({
      tryListTrackerTasks: async () =>
        ok([task({ key: 'AUTH-2' }), task({ key: 'AUTH-3', usage: usage() }), task({ key: 'AUTH-4', usage: usage() })]),
    });
    expect(l.visibleTasks.map((t) => t.key)).toEqual(['AUTH-2', 'AUTH-3', 'AUTH-4']);
    expect(l.countLine).toBe('3 tickets');
    l.aiOnly = true;
    expect(l.visibleTasks.map((t) => t.key)).toEqual(['AUTH-2']);
    expect(l.countLine).toBe('1 of 3 tickets has AI sessions');
  });

  it('says when some AI work could not be priced, so the cost is a lower bound', async () => {
    const { l } = await ready({
      tryListTrackerTasks: async () =>
        ok([task({ key: 'AUTH-3', usage: usage({ sessions: 1, api_cost: 2, api_cost_n: 1, api_cost_partial: 1 }) })]),
    });
    expect(l.costNote).toBe(
      'Some AI work could not be priced (subagents or a model with no price), so its API cost is a lower bound.',
    );
    const plain = await ready();
    expect(plain.l.costNote).toBe(null);
  });

  it('notes shared sessions', async () => {
    const { s, l } = await ready({
      tryListTrackerTasks: async () => ok([task({ key: 'AUTH-3', usage: usage({ sessions: 1, shared: 1 }) })]),
    });
    expect(l.sharedNote).toBe(
      'A session linked to several tickets counts in full for each; the sprint total counts it once.',
    );
  });
});

describe('a ticket', () => {
  it('opens to its repos and sessions; an old key shows as written', async () => {
    const { s, l, calls } = await ready();
    await s.openTicket(l, 'AUTH-420');
    expect(calls).toContain('task:AUTH-420');
    expect(s.ticket?.task.key).toBe('IAM-5');
    expect(s.linksText(s.ticket!.sessions[0])).toBe('branch · prompt ×2 · as AUTH-420');
    expect(s.backLabel).toBe('Back to All tickets');
    s.closeTicket();
    expect(s.ticket).toBe(null);
  });

  it('a failed ticket read says so', async () => {
    const { s, l } = await ready({ tryGetTrackerTask: async () => fail(404, 'task_not_found') });
    await s.openTicket(l, 'AUTH-1');
    expect(s.ticket).toBe(null);
    expect(s.ticketError).toBe('No ticket with that key in sensei.');
  });
});

describe('unlinked sessions', () => {
  it('reads them for the chosen sprint on demand', async () => {
    const { s, l, calls } = await ready();
    await l.setFilter('s13');
    await l.toggleUnlinked();
    expect(calls).toContain('unlinked:t-1/s13');
    expect(l.unlinked?.map((u) => u.id)).toEqual(['u1']);
    await l.toggleUnlinked();
    expect(l.unlinked).toBe(null);
  });

  it('has none for All tickets or the Backlog (they have no window)', async () => {
    const { s, l, calls } = await ready();
    await l.toggleUnlinked();
    await l.setFilter(BACKLOG);
    await l.toggleUnlinked();
    expect(calls.some((c) => c.startsWith('unlinked:'))).toBe(false);
  });

  it('a failed read is an error, not an empty list', async () => {
    const { s, l } = await ready({ tryListUnlinkedSessions: async () => fail(0) });
    await l.setFilter('s13');
    await l.toggleUnlinked();
    expect(l.unlinked).toBe(null);
    expect(l.unlinkedError).toBe('The sensei daemon didn’t respond.');
  });
});
