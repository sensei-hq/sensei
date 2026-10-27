// The Integrations screen's state (docs/spec/screen/workspace-integrations.md).
//
// What must never happen here: a failed read shown as "not connected", a token
// that outlives its request, a code shown raw where the screen promises words.
import { describe, it, expect } from 'vitest';
import {
  TrackerIntegration,
  errorFor,
  elapsedLabel,
  type TrackerApi,
} from './integrations-state.svelte.js';
import type { ApiResult } from '$lib/api.js';
import type { TrackerConnection } from '$lib/types.js';

const P = 'p-1';
const NOW = new Date('2026-10-18T12:00:00Z');

const row = (over: Partial<TrackerConnection> = {}): TrackerConnection => ({
  id: 't-1',
  kind: 'jira',
  level: 'sprint',
  base_url: 'https://acme.atlassian.net',
  account: 'dev@example.com',
  external_scope: 'AUTH',
  board_ref: '42',
  board_name: 'AUTH Scrum',
  status: 'connected',
  last_success_at: '2026-10-18T11:58:00Z',
  last_error: null,
  counts: { tasks: 142, sprints: 6 },
  writeback: { enabled: true, state: 'nothing_assigned', surface: 'comment', mapped: 0, assigned: 0, failing: 0, error: null },
  ...over,
});

const fail = (status: number, message: string, body?: Record<string, unknown>) =>
  ({ ok: false, error: body ? { status, message, body } : { status, message } }) as const;

/** A scripted daemon: each list call takes the next queued answer. */
function daemon(lists: Array<ApiResult<TrackerConnection[]>>, over: Partial<TrackerApi> = {}) {
  const calls: Array<[string, unknown]> = [];
  const queue = [...lists];
  const api: TrackerApi = {
    tryListTrackers: async (p) => {
      calls.push(['list', p]);
      return queue.length > 1 ? queue.shift()! : queue[0];
    },
    tryConnectTracker: async (p, kind, level, body) => {
      calls.push(['connect', { p, kind, level, body }]);
      return { ok: true, data: { tracker: row({ status: 'pending', last_success_at: null }), first_sync: { pending: true } } };
    },
    trySyncTracker: async (p, t) => {
      calls.push(['sync', { p, t }]);
      return { ok: true, data: { counts: { sprints: 6, tasks: 142, removed: 0, links: 3, skipped_members: 0 } } };
    },
    tryDisconnectTracker: async (p, t) => {
      calls.push(['disconnect', { p, t }]);
      return { ok: true, data: undefined };
    },
    trySetTrackerWriteback: async (p, t, enabled) => {
      calls.push(['writeback', { p, t, enabled }]);
      return { ok: true, data: row({ writeback: { ...row().writeback, enabled } }) };
    },
    tryGetTrackerFields: async () => ({ ok: true, data: { fields: [] } }),
    tryPutTrackerFields: async (_p, _t, fields) => ({ ok: true, data: { fields } }),
    ...over,
  };
  return { api, calls };
}

/** A manual clock for the 2 s poll: `run()` fires what is due. */
function timers() {
  let queued: Array<{ fn: () => void; ms: number }> = [];
  return {
    schedule: (fn: () => void, ms: number) => {
      const t = { fn, ms };
      queued.push(t);
      return () => {
        queued = queued.filter((q) => q !== t);
      };
    },
    pending: () => queued.map((q) => q.ms),
    async run() {
      const due = queued;
      queued = [];
      for (const t of due) t.fn();
      await new Promise((r) => setTimeout(r, 0));
    },
  };
}

function screen(lists: Array<ApiResult<TrackerConnection[]>>, over: Partial<TrackerApi> = {}) {
  const d = daemon(lists, over);
  const clock = timers();
  const s = new TrackerIntegration(d.api, P, { now: () => NOW, schedule: clock.schedule });
  return { s, ...d, clock };
}

function fill(s: TrackerIntegration) {
  s.level = 'sprint';
  s.boardUrl = 'https://acme.atlassian.net/jira/software/projects/AUTH/boards/42';
  s.email = 'dev@example.com';
  s.token = 'SENTINEL-TOKEN-XYZ';
}

describe('loading', () => {
  it('no tracker: the connect form', async () => {
    const { s } = screen([{ ok: true, data: [] }]);
    await s.load();
    expect(s.view).toBe('not_connected');
  });

  it('a tracker: the card, titled from what the daemon derived', async () => {
    const { s } = screen([{ ok: true, data: [row()] }]);
    await s.load();
    expect(s.view).toBe('connected');
    expect(s.cardTitle(s.trackers[0])).toBe('Jira · acme.atlassian.net · project AUTH · board "AUTH Scrum" (#42)');
    expect(s.cardStatus(s.trackers[0])).toBe('connected · synced 2 min ago · 6 sprints · 142 tickets');
    expect(s.showForm).toBe(false);
  });

  it('several boards: grouped by level, epics first, empty levels left out (D40)', async () => {
    const { s } = screen([
      {
        ok: true,
        data: [
          row({ id: 't-1', level: 'sprint', board_ref: '42' }),
          row({ id: 't-2', level: 'sprint', board_ref: '44' }),
          row({ id: 't-3', level: 'epic', board_ref: '43' }),
        ],
      },
    ]);
    await s.load();
    expect(s.groups.map((g) => [g.level, g.label, g.trackers.map((t) => t.id)])).toEqual([
      ['epic', 'Epics', ['t-3']],
      ['sprint', 'Sprints', ['t-1', 't-2']],
    ]);
  });

  // The rule this screen exists to keep: a failed read is never "not connected".
  it('a failed read is the daemon-error state, never the connect form', async () => {
    const { s } = screen([fail(500, 'internal', { error: 'internal' })]);
    await s.load();
    expect(s.view).toBe('daemon_error');
    expect(s.loadError).toBe('The sensei daemon hit a database error.');
  });

  it('an unreachable daemon says so', async () => {
    const { s } = screen([fail(0, 'connection refused')]);
    await s.load();
    expect(s.view).toBe('daemon_error');
    expect(s.loadError).toBe('The sensei daemon didn’t respond.');
  });
});

describe('the connect form', () => {
  it('stays disabled until level, board, email and token are all given', async () => {
    const { s } = screen([{ ok: true, data: [] }]);
    await s.load();
    expect(s.canConnect).toBe(false);
    expect(s.level).toBe(null);
    s.level = 'sprint';
    s.boardUrl = 'https://acme.atlassian.net/jira/software/projects/AUTH/boards/42';
    s.email = 'dev@example.com';
    expect(s.canConnect).toBe(false);
    s.token = '   ';
    // Whitespace is not a token.
    expect(s.canConnect).toBe(false);
    s.token = 'tok';
    expect(s.canConnect).toBe(true);
  });

  it('accepts the Advanced site and board id instead of a board URL', async () => {
    const { s } = screen([{ ok: true, data: [] }]);
    await s.load();
    s.level = 'sprint';
    s.advanced = true;
    s.siteUrl = 'https://acme.atlassian.net';
    s.email = 'dev@example.com';
    s.token = 'tok';
    expect(s.canConnect).toBe(false);
    s.boardId = '42';
    expect(s.canConnect).toBe(true);
  });

  it('sends the board URL as typed (the daemon parses it), never both forms', async () => {
    const { s, calls } = screen([{ ok: true, data: [] }]);
    await s.load();
    fill(s);
    await s.connect();
    expect(calls.find((c) => c[0] === 'connect')![1]).toEqual({
      p: P,
      kind: 'jira',
      level: 'sprint',
      body: {
        board_url: 'https://acme.atlassian.net/jira/software/projects/AUTH/boards/42',
        email: 'dev@example.com',
        api_token: 'SENTINEL-TOKEN-XYZ',
      },
    });
  });

  it('sends site + board id from Advanced', async () => {
    const { s, calls } = screen([{ ok: true, data: [] }]);
    await s.load();
    s.level = 'epic';
    s.advanced = true;
    s.siteUrl = 'https://acme.atlassian.net';
    s.boardId = '42';
    s.email = 'dev@example.com';
    s.token = 'tok';
    await s.connect();
    expect((calls.find((c) => c[0] === 'connect')![1] as { body: unknown }).body).toEqual({
      base_url: 'https://acme.atlassian.net',
      board_id: '42',
      email: 'dev@example.com',
      api_token: 'tok',
    });
  });

  it('clears the token after a success and after a failure', async () => {
    const ok = screen([{ ok: true, data: [] }]);
    await ok.s.load();
    fill(ok.s);
    await ok.s.connect();
    expect(ok.s.token).toBe('');

    const bad = screen([{ ok: true, data: [] }], {
      tryConnectTracker: async () => fail(401, 'unauthorized', { error: 'unauthorized' }),
    });
    await bad.s.load();
    fill(bad.s);
    await bad.s.connect();
    expect(bad.s.token).toBe('');
    expect(bad.s.alert?.code).toBe('unauthorized');
    expect(bad.s.view).toBe('not_connected');
  });

  it('announces the check while it runs', async () => {
    let release!: () => void;
    const { s } = screen([{ ok: true, data: [] }], {
      tryConnectTracker: () =>
        new Promise((r) => {
          release = () => r(fail(401, 'unauthorized', { error: 'unauthorized' }));
        }),
    });
    await s.load();
    fill(s);
    const done = s.connect();
    expect(s.busy).toBe('connect');
    expect(s.statusText).toBe('Checking credentials and board…');
    release();
    await done;
    expect(s.busy).toBe(null);
  });
});

describe('adding a board beside one', () => {
  it('Add a board opens an empty form at no level; Cancel closes it', async () => {
    const { s } = screen([{ ok: true, data: [row()] }]);
    await s.load();
    s.level = 'epic';
    s.startAdd();
    expect([s.showForm, s.level, s.boardUrl, s.token]).toEqual([true, null, '', '']);
    s.cancelAdd();
    expect(s.showForm).toBe(false);
  });

  it('a second board lands beside the first, and the form closes', async () => {
    const epic = row({ id: 't-2', level: 'epic', board_ref: '43', status: 'pending', last_success_at: null });
    const { s } = screen([{ ok: true, data: [row()] }, { ok: true, data: [row(), epic] }], {
      tryConnectTracker: async () => ({ ok: true, data: { tracker: epic, first_sync: { pending: true } } }),
    });
    await s.load();
    s.startAdd();
    fill(s);
    s.level = 'epic';
    await s.connect();
    expect(s.showForm).toBe(false);
    expect(s.trackers.map((t) => t.id)).toEqual(['t-1', 't-2']);
    expect(s.statusText).toBe('Connected · first sync running…');
  });
});

describe('the first sync', () => {
  it('polls every 2 s while pending, then shows the counts', async () => {
    const { s, clock } = screen([
      { ok: true, data: [] },
      { ok: true, data: [row({ status: 'pending', last_success_at: null })] },
      { ok: true, data: [row()] },
    ]);
    await s.load();
    fill(s);
    await s.connect();
    expect(s.view).toBe('connected');
    expect(s.statusText).toBe('Connected · first sync running…');
    expect(clock.pending()).toEqual([2000]);
    await clock.run();
    expect(s.statusText).toBe('Connected · first sync running…');
    await clock.run();
    expect(s.statusText).toBe('Connected · 6 sprints · 142 tickets');
    // Polling stops once the status leaves pending.
    expect(clock.pending()).toEqual([]);
  });

  it('a failed first sync keeps the connection and says why', async () => {
    const { s, clock } = screen([
      { ok: true, data: [] },
      { ok: true, data: [row({ status: 'error', last_success_at: null, last_error: 'rate_limited' })] },
    ]);
    await s.load();
    fill(s);
    await s.connect();
    await clock.run();
    expect(s.view).toBe('connected');
    expect(s.statusText).toBe(
      'Connected, but the first sync failed: Jira is rate-limiting requests. Try again in a minute.',
    );
  });

  it('polls while any board is pending', async () => {
    const { s, clock } = screen([
      { ok: true, data: [row(), row({ id: 't-2', status: 'pending', last_success_at: null })] },
    ]);
    await s.load();
    expect(clock.pending()).toEqual([2000]);
    s.dispose();
  });

  it('a screen opened while a sync is pending polls too, and stops when closed', async () => {
    const { s, clock } = screen([{ ok: true, data: [row({ status: 'pending', last_success_at: null })] }]);
    await s.load();
    expect(clock.pending()).toEqual([2000]);
    s.dispose();
    expect(clock.pending()).toEqual([]);
  });
});

describe('the connected card', () => {
  it('words each status as text (R9)', async () => {
    const at = async (r: TrackerConnection) => {
      const { s } = screen([{ ok: true, data: [r] }]);
      await s.load();
      s.dispose();
      return s.cardStatus(s.trackers[0]);
    };
    expect(await at(row({ status: 'stale', last_success_at: '2026-10-18T09:00:00Z' }))).toBe(
      'stale · last synced 3 h ago · 6 sprints · 142 tickets',
    );
    expect(await at(row({ status: 'pending', last_success_at: null }))).toBe('pending · first sync running…');
    expect(await at(row({ status: 'error', last_error: 'unauthorized' }))).toBe(
      'error · Jira rejected the email and token.',
    );
  });

  it('Sync now: a running sync is 409 busy, shown in words', async () => {
    const { s } = screen([{ ok: true, data: [row()] }], {
      trySyncTracker: async () => fail(409, 'busy', { error: 'busy' }),
    });
    await s.load();
    await s.sync('t-1');
    expect(s.alert?.text).toBe('A sync is already running.');
  });

  it('Sync now: syncs that board, then reads the connections again', async () => {
    const { s, calls } = screen([{ ok: true, data: [row(), row({ id: 't-2', board_ref: '44' })] }]);
    await s.load();
    const done = s.sync('t-2');
    expect([s.busy, s.busyTracker]).toEqual(['sync', 't-2']);
    await done;
    expect(calls.find((c) => c[0] === 'sync')![1]).toEqual({ p: P, t: 't-2' });
    expect(calls.filter((c) => c[0] === 'list')).toHaveLength(2);
    expect(s.alert).toBe(null);
  });

  it('Replace token: the form returns with site, board and email filled and no token', async () => {
    const { s } = screen([{ ok: true, data: [row({ level: 'epic' })] }]);
    await s.load();
    s.startReplace(s.trackers[0]);
    expect(s.replacing).toBe('t-1');
    expect(s.showForm).toBe(true);
    expect([s.level, s.advanced, s.siteUrl, s.boardId, s.email, s.token]).toEqual([
      'epic',
      true,
      'https://acme.atlassian.net',
      '42',
      'dev@example.com',
      '',
    ]);
    s.cancelReplace();
    expect(s.replacing).toBe(null);
    expect(s.showForm).toBe(false);
  });

  it('Disconnect asks first; Cancel does nothing; confirming removes it', async () => {
    const { s, calls } = screen([{ ok: true, data: [row()] }, { ok: true, data: [] }]);
    await s.load();
    s.askDisconnect(s.trackers[0]);
    expect(s.confirmingDisconnect?.id).toBe('t-1');
    s.cancelDisconnect();
    expect(s.confirmingDisconnect).toBe(null);
    expect(calls.some((c) => c[0] === 'disconnect')).toBe(false);

    s.askDisconnect(s.trackers[0]);
    await s.confirmDisconnect();
    expect(calls.find((c) => c[0] === 'disconnect')![1]).toEqual({ p: P, t: 't-1' });
    expect([s.view, s.confirmingDisconnect]).toEqual(['not_connected', null]);
  });

  it('a failed disconnect keeps the card and says why', async () => {
    const { s } = screen([{ ok: true, data: [row()] }], {
      tryDisconnectTracker: async () => fail(0, 'connection refused'),
    });
    await s.load();
    s.askDisconnect(s.trackers[0]);
    await s.confirmDisconnect();
    expect(s.view).toBe('connected');
    expect(s.alert?.text).toBe('The sensei daemon didn’t respond.');
  });
});

describe('errorFor (daemon code → words)', () => {
  it('names the field each refusal concerns', () => {
    expect(errorFor(422, { error: 'invalid_base_url' })).toMatchObject({ field: 'board_url' });
    expect(errorFor(422, { error: 'email_required' })).toMatchObject({ field: 'email' });
    expect(errorFor(422, { error: 'token_required' })).toMatchObject({ field: 'token' });
    expect(errorFor(401, { error: 'unauthorized' })).toMatchObject({
      text: 'Jira rejected the email and token.',
      field: 'token',
      link: { href: 'https://id.atlassian.com/manage-profile/security/api-tokens', label: 'create a new token' },
    });
  });

  it('carries the values the daemon gave', () => {
    expect(
      errorFor(404, { error: 'board_not_found', base_url: 'https://acme.atlassian.net', board_ref: '42' }).text,
    ).toBe('No board #42 on https://acme.atlassian.net.');
    expect(
      errorFor(409, { error: 'board_already_connected', workspace: { id: 'p2', name: 'billing' } }),
    ).toMatchObject({
      text: 'This board is already connected to workspace billing.',
      link: { href: '/workspace/p2/integrations', label: 'billing' },
    });
  });

  it('words every code in the table, and an unknown one with its code', () => {
    const words = (code: string) => errorFor(502, { error: code }).text;
    expect(words('forbidden')).toBe('This account can’t read that board.');
    expect(words('not_found')).toBe('Jira no longer has this board (it may have been deleted).');
    expect(words('transport')).toBe('Couldn’t reach Jira (transport). Check the site address and your connection.');
    expect(words('redirect')).toBe('Couldn’t reach Jira (redirect). Check the site address and your connection.');
    expect(words('credential_store')).toBe('The saved token can’t be read from the system keychain.');
    expect(words('token_missing')).toBe('The saved token can’t be read from the system keychain.');
    expect(errorFor(403, { error: 'forbidden_caller' }).text).toBe(
      'The sensei daemon refused this window’s request.',
    );
    expect(words('not_connected')).toBe('This workspace has no tracker connected.');
    expect(words('tracker_not_found')).toBe('This board is no longer connected to this workspace.');
    expect(words('superseded')).toBe('The board’s token changed while it synced. Sync again.');
    expect(errorFor(422, { error: 'level_required' })).toMatchObject({
      text: 'Choose what the board serves.',
      field: 'level',
    });
    expect(errorFor(422, { error: 'invalid_level' })).toMatchObject({
      text: 'A board serves epics, sprints or tasks.',
      field: 'level',
    });
    expect(words('workspace_not_found')).toBe('This workspace no longer exists in sensei.');
    expect(words('unsupported_tracker')).toBe('This tracker type isn’t available in this version of sensei.');
    expect(words('board_project_mismatch')).toBe(
      'That board belongs to a different Jira project than the one given.',
    );
    expect(words('db')).toBe('sensei hit a database error while syncing.');
    expect(words('something_new')).toBe('Unexpected error from sensei (something_new).');
  });

  it('names a ticket the board does not have', () => {
    expect(errorFor(404, { error: 'task_not_on_board', key: 'AUTH-404', board_ref: '42' }).text).toBe(
      'AUTH-404 isn’t on board #42: it may be in another Jira project, or outside the board’s filter.',
    );
    expect(errorFor(404, { error: 'task_not_found' }).text).toBe('No ticket with that key in sensei.');
  });

  it('a network failure is the daemon not responding, never "not connected"', () => {
    expect(errorFor(0, undefined).text).toBe('The sensei daemon didn’t respond.');
  });
});

describe('elapsedLabel', () => {
  it('minutes, then hours, then days', () => {
    const ago = (ms: number) => elapsedLabel(new Date(NOW.getTime() - ms).toISOString(), NOW);
    expect(ago(20_000)).toBe('just now');
    expect(ago(2 * 60_000)).toBe('2 min ago');
    expect(ago(3 * 3_600_000)).toBe('3 h ago');
    expect(ago(50 * 3_600_000)).toBe('2 days ago');
  });
});

describe('write-back (tracker-metrics-writeback W8, W-UI)', () => {
  const wb = (over: Partial<TrackerConnection['writeback']> = {}): TrackerConnection['writeback'] => ({
    enabled: true,
    state: 'active',
    surface: 'comment',
    mapped: 0,
    assigned: 2,
    failing: 0,
    error: null,
    ...over,
  });

  it('says what write-back is doing, in words, for every state', () => {
    const s = new TrackerIntegration(daemon([{ ok: true, data: [] }]).api, P);
    const line = (w: Partial<TrackerConnection['writeback']>) => s.writebackLine(row({ writeback: wb(w) }));
    expect(line({})).toBe('Writing to 2 tickets assigned to you.');
    expect(line({ assigned: 1 })).toBe('Writing to 1 ticket assigned to you.');
    expect(line({ failing: 1 })).toBe('Writing to 2 tickets assigned to you. 1 couldn’t be written.');
    expect(line({ enabled: false, state: 'disabled' })).toBe('Write-back is off.');
    expect(line({ state: 'owner_not_a_person' })).toBe(
      'This token belongs to an app account, so sensei can’t tell which tickets are yours. Nothing is written.',
    );
    expect(line({ state: 'owner_unknown' })).toBe('Checking which tickets are yours on the next sync.');
    expect(line({ state: 'nothing_assigned', assigned: 0 })).toBe('No tickets on this board are assigned to you.');
    expect(line({ state: 'owner_lookup_failed', error: 'forbidden' })).toBe(
      'Couldn’t check which tickets are yours: This account can’t read that board. Retrying on each sync.',
    );
    const passFailed = (error: string) => line({ state: 'pass_failed', error });
    expect(passFailed('token_missing')).toBe('Couldn’t read the Jira token to write back. Replace the token.');
    expect(passFailed('credential_store')).toBe('Couldn’t read the Jira token to write back. Replace the token.');
    expect(passFailed('db')).toBe('Couldn’t record what was written. Write-back retries on each sync.');
    expect(passFailed('rate_limited')).toBe('Jira is rate-limiting writes. Write-back resumes on the next sync.');
    expect(passFailed('transport')).toBe('Writing didn’t finish in time. Write-back resumes on the next sync.');
    expect(passFailed('decode')).toBe('Write-back stopped (decode).');
    expect(line({ state: 'pass_failed', error: null })).toBe('Write-back stopped (unknown).');
    expect(line({ state: 'owner_lookup_failed', error: null })).toBe(
      'Couldn’t check which tickets are yours. Retrying on each sync.',
    );
  });

  it('the board stays as it was until the daemon answers, and a second click sends nothing', async () => {
    let answer: (r: ApiResult<TrackerConnection>) => void = () => {};
    let requests = 0;
    const { api } = daemon([{ ok: true, data: [row()] }], {
      trySetTrackerWriteback: () => {
        requests += 1;
        return new Promise((r) => (answer = r));
      },
    });
    const s = new TrackerIntegration(api, P);
    await s.load();
    const first = s.toggleWriteback(s.trackers[0]);
    expect(s.busy).toBe('writeback');
    expect(s.trackers[0].writeback.enabled).toBe(true);
    await s.toggleWriteback(s.trackers[0]);
    expect(requests).toBe(1);
    answer({ ok: true, data: row({ writeback: wb({ enabled: false, state: 'disabled' }) }) });
    await first;
    expect(s.trackers[0].writeback.enabled).toBe(false);
  });

  it('switches write-back with one request and shows the tracker the daemon returns', async () => {
    const off = row({ writeback: wb({ enabled: false, state: 'disabled' }) });
    const seen: Array<[string, string, boolean]> = [];
    const { api } = daemon([{ ok: true, data: [row()] }], {
      trySetTrackerWriteback: async (p, t, enabled) => {
        seen.push([p, t, enabled]);
        return { ok: true, data: off };
      },
    });
    const s = new TrackerIntegration(api, P);
    await s.load();
    const pending = s.toggleWriteback(s.trackers[0]);
    expect(s.busy).toBe('writeback');
    await pending;
    expect(seen).toEqual([[P, 't-1', false]]);
    expect(s.busy).toBe(null);
    expect(s.trackers[0].writeback.enabled).toBe(false);
  });

  it('a refused switch leaves the board as it was and says why', async () => {
    const { api } = daemon([{ ok: true, data: [row()] }], {
      trySetTrackerWriteback: async () => fail(403, 'forbidden_caller', { error: 'forbidden_caller' }),
    });
    const s = new TrackerIntegration(api, P);
    await s.load();
    await s.toggleWriteback(s.trackers[0]);
    expect(s.trackers[0].writeback.enabled).toBe(true);
    expect(s.alert?.code).toBe('forbidden_caller');
  });
});

describe('field mapping (tracker-metrics-writeback W7, W-UI)', () => {
  type Rows = Array<{ metric: string; field_ref: string }>;
  const fieldsApi = (put: (rows: Rows) => ApiResult<{ fields: Rows }>) => {
    const sent: Rows[] = [];
    const { api } = daemon([{ ok: true, data: [row()] }], {
      tryGetTrackerFields: async () => ({
        ok: true,
        data: { fields: [{ metric: 'active_hours', field_ref: 'customfield_10050' }] },
      }),
      tryPutTrackerFields: async (_p, _t, rows) => {
        sent.push(rows);
        return put(rows) as never;
      },
    } as Partial<TrackerApi>);
    return { api, sent };
  };

  it('opens the board’s mapping, edits it and saves the whole set', async () => {
    const { api, sent } = fieldsApi((rows) => ({ ok: true, data: { fields: rows } }));
    const s = new TrackerIntegration(api, P);
    await s.load();
    await s.openFields(s.trackers[0]);
    expect(s.editingFields).toBe('t-1');
    expect(s.fieldRows).toEqual([{ metric: 'active_hours', field_ref: 'customfield_10050' }]);
    s.addFieldRow();
    s.fieldRows[1] = { metric: 'api_cost', field_ref: ' customfield_10051 ' };
    await s.saveFields();
    expect(sent).toEqual([
      [
        { metric: 'active_hours', field_ref: 'customfield_10050' },
        { metric: 'api_cost', field_ref: 'customfield_10051' },
      ],
    ]);
    expect(s.fieldError).toBe(null);
    s.removeFieldRow(0);
    expect(s.fieldRows).toEqual([{ metric: 'api_cost', field_ref: 'customfield_10051' }]);
  });

  it('a refused field is named on its row, a duplicate on the second one', async () => {
    const refusal = (body: Record<string, unknown>) => () => fail(422, String(body.error), body);
    const { api } = fieldsApi(refusal({ error: 'field_in_use', field_ref: 'customfield_10051' }));
    const s = new TrackerIntegration(api, P);
    await s.load();
    await s.openFields(s.trackers[0]);
    s.fieldRows = [
      { metric: 'active_hours', field_ref: 'customfield_10050' },
      { metric: 'api_cost', field_ref: 'customfield_10051' },
    ];
    await s.saveFields();
    expect(s.fieldError).toEqual({
      row: 1,
      text: 'That field already has values on this board. Pick an empty field.',
    });
    expect(s.alert).toBe(null);

    const dup = fieldsApi(refusal({ error: 'duplicate_mapping', metric: 'sessions' }));
    const d = new TrackerIntegration(dup.api, P);
    await d.load();
    await d.openFields(d.trackers[0]);
    d.fieldRows = [
      { metric: 'sessions', field_ref: 'customfield_1' },
      { metric: 'sessions', field_ref: 'customfield_2' },
    ];
    await d.saveFields();
    expect(d.fieldError).toEqual({ row: 1, text: 'Already mapped.' });
  });

  it('a failure that names no row is the card alert', async () => {
    const { api } = fieldsApi(() => fail(502, 'rate_limited', { error: 'rate_limited' }));
    const s = new TrackerIntegration(api, P);
    await s.load();
    await s.openFields(s.trackers[0]);
    await s.saveFields();
    expect(s.fieldError).toBe(null);
    expect(s.alert?.code).toBe('rate_limited');
  });

  it('a save while the board is busy says so and keeps the rows', async () => {
    const { api } = fieldsApi(() => fail(409, 'busy', { error: 'busy' }));
    const s = new TrackerIntegration(api, P);
    await s.load();
    await s.openFields(s.trackers[0]);
    await s.saveFields();
    expect(s.alert?.text).toBe('The board is syncing or writing back. Save again in a moment.');
    expect(s.editingFields).toBe(s.trackers[0].id);
  });
});

describe('refusedRow', () => {
  it('a duplicate names the second of its rows, whatever comes after', async () => {
    const { refusedRow } = await import('./integrations-state.svelte.js');
    const rows = [1, 2, 3].map((n) => ({ metric: 'sessions' as const, field_ref: `customfield_${n}` }));
    expect(refusedRow(rows, { error: 'duplicate_mapping', metric: 'sessions' })).toBe(1);
    const sameRef = [1, 2, 3].map(() => ({ metric: 'sessions' as const, field_ref: 'customfield_9' }));
    expect(refusedRow(sameRef, { error: 'duplicate_mapping', field_ref: 'customfield_9' })).toBe(1);
    expect(refusedRow(rows, { error: 'field_in_use', field_ref: 'customfield_3' })).toBe(2);
    expect(refusedRow(rows, { error: 'rate_limited' })).toBe(null);
  });
});
