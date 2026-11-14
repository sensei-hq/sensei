// The Delivery Metrics screen's state (docs/spec/daemon/tracker-delivery-metrics.md D2).
//
// What must never happen here: "no board" because a read failed, a 0 where the
// daemon said null, a sprint re-sorted away from the daemon's order, a ticket
// figure on a sprint screen, or a read of anything but the four D2 routes.
import { describe, it, expect, afterEach, vi } from 'vitest';
import { senseiApi } from '$lib/api.js';
import { DeliveryState, ticketHref, type DeliveryApi } from './delivery-state.svelte.js';
import { WORDS, projectSdlc, sprintSdlc, usage } from '$lib/tracker-fixtures.js';
import type { ApiResult } from '$lib/api.js';
import type { SprintUsageRow, TrackerConnection, TrackerTaskRow } from '$lib/types.js';

const P = 'p-1';

const tracker: TrackerConnection = {
  id: 't-1',
  kind: 'jira',
  base_url: 'https://acme.atlassian.net',
  account: 'dev@example.com',
  external_scope: 'AUTH',
  board_ref: '42',
  board_name: 'AUTH Scrum',
  // Jira: no GitLab sprint source, path or GitHub project address.
  sprint_source: null,
  sprint_source_changed_at: null,
  full_path: null,
  url: null,
  iteration_field: null,
  level: 'sprint',
  status: 'connected',
  last_success_at: '2026-10-18T11:58:00Z',
  last_error: null,
  counts: { tasks: 3, sprints: 2 },
  writeback: { enabled: true, state: 'nothing_assigned', surface: 'comment', mapped: 0, assigned: 0, failing: 0, not_private: 0, supports: { comment: true, fields: true }, error: null },
  sdlc_project: projectSdlc({ predictability: 12.5, tokens_per_ticket: 1500, prompts_per_ticket: 2, reasons: {} }),
};

const sprint = (over: Partial<SprintUsageRow> = {}): SprintUsageRow => ({
  id: 's12',
  name: 'Sprint 12',
  state: 'closed',
  start_at: '2026-06-15T12:00:00Z',
  end_at: '2026-06-25T12:00:00Z',
  complete_at: '2026-06-25T13:00:00Z',
  tasks: 5,
  candidates_in_window: 3,
  linked_sprint: usage({ sessions: 2, active_secs: 3840, active_n: 2, api_cost: 1.24, api_cost_n: 2, metered_cost: 9, metered_cost_n: 2 }),
  linked_other: usage({ sessions: 1 }),
  unlinked: usage(),
  reason: null,
  sdlc: sprintSdlc({
    commitment_reliability: 80,
    total_completion: 90,
    scope_change: 0,
    carry_over_rate: 20,
    unplanned_work_ratio: 10,
    velocity: 21,
    points_commitment_reliability: 75,
    facts: { committed: 10, added: 2, removed: 1, completed_committed: 8, completed_added: 1, carried_over: 2, unknown_status: 0 },
    reasons: {},
  }),
  ...over,
});

const future = sprint({
  id: 's14',
  name: 'Sprint 14',
  state: 'future',
  start_at: null,
  end_at: null,
  complete_at: null,
  candidates_in_window: null,
  linked_sprint: null,
  linked_other: null,
  unlinked: null,
  reason: 'not_started',
  sdlc: sprintSdlc(),
});

const task = (over: Partial<TrackerTaskRow> = {}): TrackerTaskRow => ({
  key: 'AUTH-2',
  summary: 'Login',
  issue_type: 'Story',
  status: 'Done',
  current_sprint: { id: 's12', name: 'Sprint 12' },
  sprints: [{ id: 's12', name: 'Sprint 12' }],
  removed: false,
  assigned_to_me: null,
  writeback_status: null,
  writeback_at: null,
  usage: usage(),
  sdlc: {} as TrackerTaskRow['sdlc'],
  ...over,
});

const ok = <T,>(data: T): ApiResult<T> => ({ ok: true, data });
const fail = (status: number) => ({ ok: false, error: { status, message: 'refused' } }) as const;

function daemon(over: Partial<DeliveryApi> = {}) {
  const calls: string[] = [];
  const api: DeliveryApi = {
    tryListTrackers: async (w) => (calls.push(`trackers ${w}`), ok([tracker])),
    tryListSprints: async (w, t) => (calls.push(`sprints ${w} ${t}`), ok([future, sprint(), sprint({ id: 's11', name: 'Sprint 11' })])),
    tryListTrackerTasks: async (w, t, s) => (calls.push(`tasks ${w} ${t} ${s}`), ok([task()])),
    tryGetReasonCodes: async (d) => (calls.push(`reasons ${d}`), ok({ domain: d, reasons: WORDS })),
    ...over,
  };
  return { api, calls };
}

async function ready(over: Partial<DeliveryApi> = {}) {
  const { api, calls } = daemon(over);
  const s = new DeliveryState(api, P);
  await s.load();
  return { s, calls, b: s.boards[0] };
}

describe('load', () => {
  it('reads the boards and the sdlc_metrics words, then each sprint board’s sprints', async () => {
    const { s, calls } = await ready();
    expect(s.view).toBe('ready');
    expect(calls).toEqual([`trackers ${P}`, 'reasons sdlc_metrics', `sprints ${P} t-1`]);
  });

  it('no sprint board is the not-connected state', async () => {
    const { s } = await ready({ tryListTrackers: async () => ok([{ ...tracker, level: 'epic' }]) });
    expect(s.view).toBe('not_connected');
  });

  it('any failed read is the daemon error, never "no board" or an empty table', async () => {
    for (const over of [
      { tryListTrackers: async () => fail(500) },
      { tryGetReasonCodes: async () => fail(500) },
      { tryListSprints: async () => fail(500) },
    ] as Partial<DeliveryApi>[]) {
      const { s } = await ready(over);
      expect(s.view).toBe('daemon_error');
      expect(s.loadError).not.toBe(null);
      expect(s.boards).toEqual([]);
    }
  });

  it('Retry after a failure recovers', async () => {
    let n = 0;
    const { s } = await ready({ tryListSprints: async () => (++n === 1 ? fail(500) : ok([sprint()])) });
    expect(s.view).toBe('daemon_error');
    await s.load();
    expect(s.view).toBe('ready');
  });
});

describe('a board', () => {
  it("shows the board's project row", async () => {
    const { b } = await ready();
    expect(b.title).toBe('AUTH Scrum');
    expect(b.project.map((f) => f.text)).toEqual(['12.5 pts', '1.5k', '2.0']);
  });

  it("keeps the daemon's sprint order, never re-sorting", async () => {
    const { b } = await ready();
    expect(b.rows.map((r) => r.id)).toEqual(['s14', 's12', 's11']);
  });

  it('a sprint row shows its seven figures and your sessions’ active time, fresh tokens and API cost', async () => {
    const { b } = await ready();
    const row = b.rows.find((r) => r.id === 's12')!;
    expect(row.figures.map((f) => f.text)).toEqual(['80%', '90%', '0%', '20%', '10%', '21 pts', '75%']);
    expect(row.effort).toEqual({ kind: 'cells', active: '1h 04m', fresh: '—', cost: '$1.24' });
  });

  it('dates run from the start to the close (else the planned end); a sprint with none says so', async () => {
    const { b } = await ready({
      tryListSprints: async () =>
        ok([
          future,
          sprint({ id: 'early', complete_at: '2026-06-20T12:00:00Z' }),
          sprint({ id: 'open', state: 'active', complete_at: null }),
        ]),
    });
    expect(b.rows.find((r) => r.id === 'early')!.dates).toBe('Jun 15 – Jun 20');
    expect(b.rows.find((r) => r.id === 'open')!.dates).toBe('Jun 15 – Jun 25');
    expect(b.rows.find((r) => r.id === 's14')!.dates).toBe('No dates');
  });

  it("a sprint with no window shows Tickets' words for its reason in the effort cells, never zeros", async () => {
    const { b } = await ready();
    const row = b.rows.find((r) => r.id === 's14')!;
    expect(row.effort).toEqual({ kind: 'reason', text: 'Not started. Usage counts from the sprint’s start.' });
    expect(row.figures[0]).toMatchObject({ text: 'Ticket history not read yet', reason: true });
  });
});

describe('a selected sprint', () => {
  it('shows its fact counts and its tickets, each linking to the ticket on Tickets', async () => {
    const { b, calls } = await ready();
    expect(b.selected).toBe(null);
    await b.select('s12');
    expect(calls.at(-1)).toBe(`tasks ${P} t-1 s12`);
    expect(b.facts).toEqual([
      { key: 'committed', label: 'committed', value: '10' },
      { key: 'added', label: 'added', value: '2' },
      { key: 'removed', label: 'removed', value: '1' },
      { key: 'completed_committed', label: 'completed committed', value: '8' },
      { key: 'completed_added', label: 'completed added', value: '1' },
      { key: 'carried_over', label: 'carried over', value: '2' },
    ]);
    expect(b.factsNote).toBe(null);
    expect(b.tickets).toEqual([
      { key: 'AUTH-2', title: 'Login', href: `/workspace/${P}/tickets?board=t-1&ticket=AUTH-2` },
    ]);
  });

  it('names tickets with an unknown status', async () => {
    const { b } = await ready({
      tryListSprints: async () =>
        ok([sprint({ sdlc: { ...sprint().sdlc, facts: { ...sprint().sdlc.facts!, unknown_status: 2 } } })]),
    });
    await b.select('s12');
    expect(b.factsNote).toBe('2 tickets with an unknown status');
  });

  it("a sprint with no replay says why instead of showing counts", async () => {
    const { b } = await ready();
    await b.select('s14');
    expect(b.facts).toBe(null);
    expect(b.factsNote).toBe('Ticket history not read yet');
  });

  it('a failed ticket read is an error, never an empty list', async () => {
    const { b } = await ready({ tryListTrackerTasks: async () => fail(500) });
    await b.select('s12');
    expect(b.tickets).toBe(null);
    expect(b.ticketsError).not.toBe(null);
  });

  it('a slower earlier selection is dropped', async () => {
    let release!: () => void;
    const gate = new Promise<void>((r) => (release = r));
    const { b } = await ready({
      tryListTrackerTasks: async (_w, _t, s) => {
        if (s === 's11') await gate;
        return ok([task({ key: s === 's11' ? 'OLD-1' : 'AUTH-2' })]);
      },
    });
    const first = b.select('s11');
    await b.select('s12');
    release();
    await first;
    expect(b.selected?.id).toBe('s12');
    expect(b.tickets?.map((t) => t.key)).toEqual(['AUTH-2']);
  });
});

describe('ticketHref', () => {
  it('encodes the board and the key', () => {
    expect(ticketHref('p 1', 't/1', 'A&B-1')).toBe('/workspace/p%201/tickets?board=t%2F1&ticket=A%26B-1');
  });
});

describe('only the four D2 reads (gate 5)', () => {
  afterEach(() => vi.unstubAllGlobals());

  it('loading and selecting a sprint request only trackers, sprints, tasks and the sdlc_metrics words — never /api/dojo/*', async () => {
    const paths: string[] = [];
    const bodies: Array<[RegExp, unknown]> = [
      [/\/trackers$/, [tracker]],
      [/\/sprints$/, [sprint()]],
      [/\/tasks\?sprint=/, [task()]],
      [/\/api\/reason-codes\//, { domain: 'sdlc_metrics', reasons: WORDS }],
    ];
    vi.stubGlobal('fetch', async (url: string) => {
      const path = new URL(url).pathname + new URL(url).search;
      paths.push(path);
      const body = bodies.find(([re]) => re.test(path))?.[1];
      return new Response(JSON.stringify(body ?? null), { status: body ? 200 : 404 });
    });
    const s = new DeliveryState(senseiApi(7744), P);
    await s.load();
    await s.boards[0].select('s12');
    expect(s.view).toBe('ready');
    const allowed = [
      new RegExp(`^/api/workspaces/${P}/trackers$`),
      new RegExp(`^/api/workspaces/${P}/trackers/t-1/sprints$`),
      new RegExp(`^/api/workspaces/${P}/trackers/t-1/tasks\\?sprint=s12$`),
      /^\/api\/reason-codes\/sdlc_metrics$/,
    ];
    expect(paths.length).toBe(4);
    for (const p of paths) {
      expect(p.startsWith('/api/dojo/'), p).toBe(false);
      expect(allowed.some((re) => re.test(p)), p).toBe(true);
    }
  });
});
