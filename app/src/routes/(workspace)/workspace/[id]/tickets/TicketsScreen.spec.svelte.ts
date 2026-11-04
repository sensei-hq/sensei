// @vitest-environment jsdom
//
// The Sprints screen as a user meets it: sprint rows that are buttons, numbers
// that say "—" when unknown, a ticket that opens to its sessions, and failures
// that never pass for "no tracker connected".
import { describe, it, expect, afterEach } from 'vitest';
import { flushSync } from 'svelte';
import axe from 'axe-core';
import { mountComponent } from '$lib/test-mount.js';
import { replayHref } from '$lib/sessions-digest.js';
import TicketsScreen from './TicketsScreen.svelte';
import { TicketsState, type SprintsApi } from './tickets-state.svelte.js';
import { WORDS, projectSdlc, sprintSdlc, ticketSdlc, usage } from '$lib/tracker-fixtures.js';
import type { SprintUsageRow, TrackerConnection } from '$lib/types.js';

let cleanup: Array<() => void> = [];
afterEach(() => {
  cleanup.forEach((fn) => fn());
  cleanup = [];
});


const active: SprintUsageRow = {
  id: 's13',
  name: 'Sprint 13',
  state: 'active',
  start_at: '2026-10-01T12:00:00Z',
  end_at: '2026-10-15T12:00:00Z',
  complete_at: null,
  tasks: 2,
  candidates_in_window: 1,
  linked_sprint: usage({ sessions: 1, active_secs: 600, active_n: 1 }),
  linked_other: usage(),
  unlinked: usage(),
  reason: null,
  sdlc: sprintSdlc(),
};

const future: SprintUsageRow = {
  ...active,
  id: 's14',
  name: 'Sprint 14',
  state: 'future',
  candidates_in_window: null,
  linked_sprint: null,
  linked_other: null,
  unlinked: null,
  reason: 'not_started',
};

function api(over: Partial<SprintsApi> = {}): SprintsApi {
  return {
    tryListTrackers: async () => ({
      ok: true,
      data: [
        {
          id: 't',
          kind: 'jira',
          base_url: 'https://acme.atlassian.net',
          account: 'a@b.c',
          external_scope: 'AUTH',
          board_ref: '42',
          board_name: 'AUTH Scrum',
          level: 'sprint',
          status: 'connected',
          last_success_at: null,
          last_error: null,
          counts: { tasks: 2, sprints: 2 },
          writeback: { enabled: true, state: 'nothing_assigned', surface: 'comment', mapped: 0, assigned: 0, failing: 0, error: null },
          sdlc_project: projectSdlc(),
        },
      ],
    }),
    tryListSprints: async () => ({ ok: true, data: [future, active] }),
    tryListTrackerTasks: async () => ({
      ok: true,
      data: [
        {
          key: 'AUTH-2',
          summary: 'Login',
          issue_type: 'Story',
          status: 'To Do',
          current_sprint: { id: 's13', name: 'Sprint 13' },
          sprints: [{ id: 's13', name: 'Sprint 13' }],
          removed: false,
          assigned_to_me: null,
          writeback_status: null,
          writeback_at: null,
          usage: usage({ sessions: 1, by_source: { branch: 1, prompt: 0, fetch: 0 }, active_secs: 600, active_n: 1 }),
          sdlc: ticketSdlc(),
        },
      ],
    }),
    tryGetTrackerTask: async () => ({
      ok: true,
      data: {
        task: { key: 'AUTH-2', summary: 'Login', issue_type: 'Story', status: 'To Do', removed: false, assigned_to_me: null, writeback_status: null, writeback_at: null },
        usage: usage({ sessions: 1 }),
        sdlc: ticketSdlc({ compile_defects: 1, runs_unknown: 2 }),
        by_repo: [{ repository_id: 'r1', name: 'backend', usage: usage({ sessions: 1 }) }],
        sessions: [
          { id: 'sess-1', started_at: '2026-10-02T09:00:00Z', links: [{ source: 'branch', matched_key: 'AUTH-2', mentions: 1 }] },
        ],
      },
    }),
    tryListUnlinkedSessions: async () => ({ ok: true, data: [] }),
    tryGetReasonCodes: async (domain) => ({ ok: true, data: { domain, reasons: WORDS } }),
    tryGetBacklog: async () => ({ ok: true, data: { tickets: 1, usage: usage({ sessions: 1 }) } }),
    tryListBacklogTasks: async () => ({
      ok: true,
      data: [
        {
          key: 'P-783',
          summary: 'Testing without sprint',
          issue_type: 'Story',
          status: 'To-Do',
          current_sprint: null,
          sprints: [],
          removed: false,
          assigned_to_me: null,
          writeback_status: null,
          writeback_at: null,
          usage: usage({ sessions: 1 }),
          sdlc: ticketSdlc(),
        },
      ],
    }),
    ...over,
  };
}

async function open(over: Partial<SprintsApi> = {}) {
  const screen = new TicketsState(api(over), 'p-1');
  await screen.load();
  const m = mountComponent(TicketsScreen, { screen, workspaceId: 'p-1' });
  cleanup.push(m.destroy);
  flushSync();
  const q = <T extends Element = HTMLElement>(sel: string) => m.container.querySelector(sel) as T | null;
  return { screen, m, q };
}

const settle = async () => {
  await new Promise((r) => setTimeout(r, 0));
  flushSync();
};

describe('ready', () => {
  it('the header says the numbers are written back to tickets assigned to you (W-UI)', async () => {
    const { m } = await open();
    expect(m.container.textContent).toContain(
      'Read from Jira. On tickets assigned to you, sensei writes these numbers back, as set on Integrations.',
    );
    expect(m.container.textContent).not.toContain('nothing is changed there');
  });

  it('starts with every ticket and a sprint filter, not a sprint list', async () => {
    const { q, m } = await open();
    expect(m.container.querySelectorAll('button[data-sprint]').length).toBe(0);
    expect(q('[data-filter="sprint"]')).not.toBe(null);
    expect(q('[data-ticket="AUTH-2"]')).not.toBe(null);
    expect(q('[data-strip]')).toBe(null);
  });

  it('one lane needs no heading; two sprint boards are two lanes side by side, each headed (D40)', async () => {
    const one = await open();
    expect(one.m.container.querySelectorAll('section[data-lane]').length).toBe(1);
    expect(one.q('section[data-lane] h2')).toBe(null);

    const base = (await api().tryListTrackers('p-1')) as { ok: true; data: TrackerConnection[] };
    const mobile = { ...base.data[0], id: 't2', board_ref: '44', board_name: 'Mobile Scrum' };
    const { m } = await open({ tryListTrackers: async () => ({ ok: true, data: [base.data[0], mobile] }) });
    const lanes = [...m.container.querySelectorAll('section[data-lane]')];
    expect(lanes.map((l) => l.querySelector('h2')?.textContent?.trim())).toEqual(['AUTH Scrum', 'Mobile Scrum']);
    for (const lane of lanes) {
      expect(lane.querySelector('[data-filter="sprint"]')).not.toBe(null);
      const input = lane.querySelector<HTMLInputElement>('input[data-filter="ai-only"]')!;
      expect(lane.querySelector(`label[for="${input.id}"]`)).not.toBe(null);
    }
    expect(new Set(lanes.map((l) => l.querySelector('input[data-filter="ai-only"]')!.id)).size).toBe(2);
  });

  it('each row names its sprint', async () => {
    const { q } = await open();
    expect(q('[data-ticket="AUTH-2"] [data-cell="sprint"]')!.textContent!.trim()).toBe('Sprint 13');
  });

  it('an AI-only switch narrows the list and says how many are shown', async () => {
    const { q } = await open({
      tryListTrackerTasks: async () => ({
        ok: true,
        data: [
          { key: 'AUTH-2', summary: 'Login', issue_type: 'Story', status: 'To Do', current_sprint: { id: 's13', name: 'Sprint 13' }, sprints: [], removed: false, assigned_to_me: null, writeback_status: null, writeback_at: null, usage: usage({ sessions: 1 }), sdlc: ticketSdlc() },
          { key: 'AUTH-9', summary: 'Idle', issue_type: 'Story', status: 'To Do', current_sprint: null, sprints: [], removed: false, assigned_to_me: null, writeback_status: null, writeback_at: null, usage: usage(), sdlc: ticketSdlc() },
        ],
      }),
    });
    const toggle = q<HTMLInputElement>('input[data-filter="ai-only"]')!;
    expect(q(`label[for="${toggle.id}"]`)!.textContent).toContain('Only tickets with AI sessions');
    expect(q('[data-count]')!.textContent).toContain('2 tickets');
    toggle.click();
    flushSync();
    expect(q('[data-ticket="AUTH-9"]')).toBe(null);
    expect(q('[data-count]')!.textContent).toContain('1 of 2 tickets has AI sessions');
  });

  it('labels time as active AI time and never as hours', async () => {
    const { m } = await open();
    const text = m.container.textContent ?? '';
    expect(text).toMatch(/active AI time/i);
    expect(text).not.toMatch(/\bhours\b|time spent/i);
  });

  it('shows unknown numbers as —, never 0', async () => {
    const { q, m } = await open();
    const row = q('[data-ticket="AUTH-2"]')!;
    expect(row.querySelector('[data-cell="fresh"]')!.textContent!.trim()).toBe('—');
    expect(row.querySelector('[data-cell="cost"]')!.textContent!.trim()).toBe('—');
    expect(m.container.textContent).toMatch(/API cost/);
    expect(row.querySelector('[data-cell="active"]')!.textContent!.trim()).toBe('10m');
    expect(row.querySelector('[data-cell="cycle"]')!.textContent!.trim()).toBe('—');
  });

  it('a future sprint, filtered to, says it has not started, with no zeros', async () => {
    const { q, screen } = await open();
    await screen.lanes[0].setFilter('s14');
    flushSync();
    const strip = q('[data-strip]')!;
    expect(strip.textContent).toContain('Not started.');
    expect(strip.textContent).not.toMatch(/\b0m\b|\$0/);
  });

  it('a ticket shows Claude and you, session time and cycle time', async () => {
    const { q } = await open({
      tryGetTrackerTask: async () => ({
        ok: true,
        data: {
          task: { key: 'AUTH-2', summary: 'Login', issue_type: 'Story', status: 'To Do', removed: false, assigned_to_me: null, writeback_status: null, writeback_at: null },
          usage: usage({
            sessions: 2,
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
          sdlc: ticketSdlc(),
          by_repo: [],
          sessions: [],
        },
      }),
    });
    q<HTMLButtonElement>('[data-ticket="AUTH-2"]')!.click();
    await settle();
    const time = (k: string) => q(`[data-view="ticket"] [data-time="${k}"]`)!.textContent!.trim();
    expect([time('claude'), time('you'), time('wall'), time('cycle')]).toEqual(['30m', '10m', '50m', '2d 7h']); // the shared duration format past a day (lib/sessions-digest)
  });

  it('a ticket with no measured time shows — for each, never 0m', async () => {
    const { q } = await open();
    q<HTMLButtonElement>('[data-ticket="AUTH-2"]')!.click();
    await settle();
    const detail = q('[data-view="ticket"]')!;
    for (const k of ['claude', 'you', 'wall', 'cycle']) {
      expect(detail.querySelector(`[data-time="${k}"]`)!.textContent!.trim()).toBe('—');
    }
  });

  it('a ticket opens to its repos and sessions, each linking to the replay', async () => {
    const { q } = await open();
    q<HTMLButtonElement>('[data-ticket="AUTH-2"]')!.click();
    await settle();
    const detail = q('[data-view="ticket"]')!;
    expect(detail.textContent).toContain('backend');
    const link = detail.querySelector<HTMLAnchorElement>('a[data-session="sess-1"]')!;
    expect(link.getAttribute('href')).toBe(replayHref('sess-1'));
    q<HTMLButtonElement>('[data-action="back"]')!.click();
    flushSync();
    expect(q('[data-view="ticket"]')).toBe(null);
  });
});

describe('SDLC figures (tracker-sdlc-metrics S8)', () => {
  it("a ticket's SDLC group shows each null as its reason's words and marks defects while runs are unknown", async () => {
    const { q } = await open();
    q<HTMLButtonElement>('[data-ticket="AUTH-2"]')!.click();
    await settle();
    const group = q('[data-section="sdlc"]')!;
    expect([...group.querySelectorAll('[data-sdlc-row]')].map((r) => r.getAttribute('data-sdlc-row'))).toEqual([
      'cost',
      'defects',
      'timing',
      'rework',
    ]);
    expect(q('[data-sdlc="effort_cost"]')!.textContent).toBe('No AI sessions linked');
    expect(q('[data-sdlc="compile_defects"]')!.textContent).toBe('≥ 1');
    expect(q('[data-sdlc-row="defects"] [data-sdlc-note]')!.textContent).toBe('2 runs with unknown outcome');
    expect(group.textContent).not.toMatch(/no_sessions|null|NaN/);
  });

  it("the lane header shows the board's row on All tickets and the sprint's row on a sprint", async () => {
    const { q, screen } = await open();
    expect(q('[data-sdlc-lane="project"] [data-sdlc="predictability"]')!.textContent).toBe('Too few sprints');
    await screen.lanes[0].setFilter('s13');
    flushSync();
    expect(q('[data-sdlc-lane="project"]')).toBe(null);
    expect(q('[data-sdlc-lane="sprint"] [data-sdlc="commitment_reliability"]')!.textContent).toBe(
      'Ticket history not read yet',
    );
  });

  it('a failed read of the reason words is the daemon error with Retry, which recovers', async () => {
    let calls = 0;
    const { q } = await open({
      tryGetReasonCodes: async (domain) =>
        ++calls === 1
          ? { ok: false, error: { status: 0, message: 'Network error' } }
          : { ok: true, data: { domain, reasons: WORDS } },
    });
    expect(q('[data-view="daemon_error"]')).not.toBe(null);
    q<HTMLButtonElement>('[data-action="retry"]')!.click();
    await settle();
    await settle();
    expect(q('[data-view="daemon_error"]')).toBe(null);
    expect(q('[data-sdlc-lane="project"]')).not.toBe(null);
  });

  it('a real 0 renders as 0, never as a reason or —', async () => {
    const { q } = await open({
      tryListTrackers: async () => ({
        ok: true,
        data: [
          {
            ...(await api().tryListTrackers('p-1').then((r) => (r.ok ? r.data[0] : null)))!,
            sdlc_project: projectSdlc({ predictability: 0, tokens_per_ticket: 0, prompts_per_ticket: 0, reasons: {} }),
          },
        ],
      }),
    });
    expect(q('[data-sdlc="predictability"]')!.textContent).toBe('0 pts');
    expect(q('[data-sdlc="tokens_per_ticket"]')!.textContent).toBe('0');
    expect(q('[data-sdlc="prompts_per_ticket"]')!.textContent).toBe('0.0');
  });
});

describe('backlog', () => {
  it('filtering to Backlog lists the tickets in no sprint, with their total', async () => {
    const { q, screen } = await open();
    await screen.lanes[0].setFilter('backlog');
    flushSync();
    expect(q('[data-strip]')!.textContent).toContain('1 ticket in no sprint');
    expect(q('[data-ticket="P-783"] [data-cell="sprint"]')!.textContent!.trim()).toBe('Backlog');
  });
});

describe('not connected', () => {
  it('points to Integrations', async () => {
    const { q } = await open({
      tryListTrackers: async () => ({
        ok: true,
        data: [],
      }),
    });
    const view = q('[data-view="not_connected"]')!;
    expect(view.textContent).toContain('This workspace has no sprint board connected.');
    expect(view.querySelector('a')!.getAttribute('href')).toBe('/workspace/p-1/integrations');
  });
});

describe('daemon error', () => {
  it('shows the failure and Retry, never "no tracker connected"', async () => {
    const { q, m } = await open({
      tryListSprints: async () => ({ ok: false, error: { status: 0, message: 'refused' } }),
    });
    expect(q('[data-view="daemon_error"]')!.textContent).toContain('The sensei daemon didn’t respond.');
    expect(q('[data-action="retry"]')).not.toBe(null);
    expect(m.container.textContent).not.toContain('no tracker connected');
  });
});

describe('accessibility (axe, structural rules; contrast is checked in the e2e scan)', () => {
  const states: Array<[string, Partial<SprintsApi>, boolean]> = [
    ['ready', {}, false],
    ['ticket', {}, true],
    ['not connected', { tryListTrackers: async () => ({ ok: true, data: [] }) }, false],
    ['daemon error', { tryListSprints: async () => ({ ok: false, error: { status: 0, message: 'x' } }) }, false],
  ];
  for (const [name, over, ticket] of states) {
    it(`${name}: no serious or critical violations`, async () => {
      const { m, q } = await open(over);
      if (ticket) {
        q<HTMLButtonElement>('[data-ticket="AUTH-2"]')!.click();
        await settle();
      }
      const result = await axe.run(m.container, {
        runOnly: { type: 'tag', values: ['wcag2a', 'wcag2aa'] },
        rules: { 'color-contrast': { enabled: false } },
      });
      const bad = result.violations
        .filter((v) => v.impact === 'serious' || v.impact === 'critical')
        .map((v) => `${v.id}: ${v.nodes.map((n) => n.target.join(' ')).join(', ')}`);
      expect(bad).toEqual([]);
    });
  }
});
