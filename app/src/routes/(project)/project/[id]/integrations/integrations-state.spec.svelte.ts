// The Integrations screen's state (docs/spec/screen/project-integrations.md).
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
  base_url: 'https://acme.atlassian.net',
  account: 'dev@example.com',
  external_scope: 'AUTH',
  board_ref: '42',
  board_name: 'AUTH Scrum',
  status: 'connected',
  last_success_at: '2026-10-18T11:58:00Z',
  last_error: null,
  counts: { tasks: 142, sprints: 6 },
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
    tryConnectTracker: async (p, kind, body) => {
      calls.push(['connect', { p, kind, body }]);
      return { ok: true, data: { tracker: row({ status: 'pending', last_success_at: null }), first_sync: { pending: true } } };
    },
    trySyncTracker: async (p) => {
      calls.push(['sync', p]);
      return { ok: true, data: { counts: { sprints: 6, tasks: 142, removed: 0, links: 3, skipped_members: 0 } } };
    },
    tryDisconnectTracker: async (p) => {
      calls.push(['disconnect', p]);
      return { ok: true, data: undefined };
    },
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
    expect(s.cardTitle).toBe('Jira · acme.atlassian.net · project AUTH · board "AUTH Scrum" (#42)');
    expect(s.cardStatus).toBe('connected · synced 2 min ago · 6 sprints · 142 tickets');
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
  it('stays disabled until board, email and token are all given', async () => {
    const { s } = screen([{ ok: true, data: [] }]);
    await s.load();
    expect(s.canConnect).toBe(false);
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
      return s.cardStatus;
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
    await s.sync();
    expect(s.alert?.text).toBe('A sync is already running.');
  });

  it('Sync now: success reads the connection again', async () => {
    const { s, calls } = screen([{ ok: true, data: [row()] }]);
    await s.load();
    await s.sync();
    expect(calls.filter((c) => c[0] === 'list')).toHaveLength(2);
    expect(s.alert).toBe(null);
  });

  it('Replace token: the form returns with site, board and email filled and no token', async () => {
    const { s } = screen([{ ok: true, data: [row()] }]);
    await s.load();
    s.startReplace();
    expect(s.replacing).toBe(true);
    expect([s.advanced, s.siteUrl, s.boardId, s.email, s.token]).toEqual([
      true,
      'https://acme.atlassian.net',
      '42',
      'dev@example.com',
      '',
    ]);
    s.cancelReplace();
    expect(s.replacing).toBe(false);
  });

  it('Disconnect asks first; Cancel does nothing; confirming removes it', async () => {
    const { s, calls } = screen([{ ok: true, data: [row()] }]);
    await s.load();
    s.askDisconnect();
    expect(s.confirmingDisconnect).toBe(true);
    s.cancelDisconnect();
    expect(s.confirmingDisconnect).toBe(false);
    expect(calls.some((c) => c[0] === 'disconnect')).toBe(false);

    s.askDisconnect();
    await s.confirmDisconnect();
    expect(calls.some((c) => c[0] === 'disconnect')).toBe(true);
    expect([s.view, s.confirmingDisconnect]).toEqual(['not_connected', false]);
  });

  it('a failed disconnect keeps the card and says why', async () => {
    const { s } = screen([{ ok: true, data: [row()] }], {
      tryDisconnectTracker: async () => fail(0, 'connection refused'),
    });
    await s.load();
    s.askDisconnect();
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
      errorFor(409, { error: 'board_already_connected', project: { id: 'p2', name: 'billing' } }),
    ).toMatchObject({
      text: 'This board is already connected to project billing.',
      link: { href: '/project/p2/integrations', label: 'billing' },
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
    expect(words('not_connected')).toBe('This project has no tracker connected.');
    expect(words('project_not_found')).toBe('This project no longer exists in sensei.');
    expect(words('unsupported_tracker')).toBe('This tracker type isn’t available in this version of sensei.');
    expect(words('board_project_mismatch')).toBe(
      'That board belongs to a different Jira project than the one given.',
    );
    expect(words('db')).toBe('sensei hit a database error while syncing.');
    expect(words('something_new')).toBe('Unexpected error from sensei (something_new).');
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
