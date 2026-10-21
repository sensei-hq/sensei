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
import SprintsScreen from './SprintsScreen.svelte';
import { SprintsState, type SprintsApi } from './sprints-state.svelte.js';
import type { SprintUsageRow, TrackerUsage } from '$lib/types.js';

let cleanup: Array<() => void> = [];
afterEach(() => {
  cleanup.forEach((fn) => fn());
  cleanup = [];
});

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
          status: 'connected',
          last_success_at: null,
          last_error: null,
          counts: { tasks: 2, sprints: 2 },
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
          usage: usage({ sessions: 1, by_source: { branch: 1, prompt: 0 }, active_secs: 600, active_n: 1 }),
        },
      ],
    }),
    tryGetTrackerTask: async () => ({
      ok: true,
      data: {
        task: { key: 'AUTH-2', summary: 'Login', issue_type: 'Story', status: 'To Do', removed: false },
        usage: usage({ sessions: 1 }),
        by_repo: [{ repository_id: 'r1', name: 'backend', usage: usage({ sessions: 1 }) }],
        sessions: [
          { id: 'sess-1', started_at: '2026-10-02T09:00:00Z', links: [{ source: 'branch', matched_key: 'AUTH-2', mentions: 1 }] },
        ],
      },
    }),
    tryListUnlinkedSessions: async () => ({ ok: true, data: [] }),
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
          usage: usage({ sessions: 1 }),
        },
      ],
    }),
    ...over,
  };
}

async function open(over: Partial<SprintsApi> = {}) {
  const screen = new SprintsState(api(over), 'p-1');
  await screen.load();
  const m = mountComponent(SprintsScreen, { screen, projectId: 'p-1' });
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
  it('lists sprints as buttons and marks the selected one', async () => {
    const { q, m } = await open();
    const rows = Array.from(m.container.querySelectorAll<HTMLButtonElement>('button[data-sprint]'));
    expect(rows.map((r) => r.dataset.sprint)).toEqual(['s14', 's13', 'backlog']);
    expect(q('button[data-sprint="s13"]')!.getAttribute('aria-current')).toBe('true');
    expect(q('button[data-sprint="s14"]')!.getAttribute('aria-current')).toBe(null);
  });

  it('labels time as active AI time and never as hours', async () => {
    const { m } = await open();
    const text = m.container.textContent ?? '';
    expect(text).toMatch(/active AI time/i);
    expect(text).not.toMatch(/\bhours\b|time spent/i);
  });

  it('shows unknown numbers as —, never 0', async () => {
    const { q } = await open();
    const row = q('[data-ticket="AUTH-2"]')!;
    expect(row.querySelector('[data-cell="fresh"]')!.textContent!.trim()).toBe('—');
    expect(row.querySelector('[data-cell="cost"]')!.textContent!.trim()).toBe('—');
    expect(row.querySelector('[data-cell="active"]')!.textContent!.trim()).toBe('10m');
  });

  it('a future sprint says it has not started, with no zeros', async () => {
    const { q, screen } = await open();
    q<HTMLButtonElement>('button[data-sprint="s14"]')!.click();
    await settle();
    expect(screen.selected?.id).toBe('s14');
    const strip = q('[data-strip]')!;
    expect(strip.textContent).toContain('Not started.');
    expect(strip.textContent).not.toMatch(/\b0m\b|\$0/);
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

describe('backlog', () => {
  it('is a selectable row that lists the tickets in no sprint', async () => {
    const { q } = await open();
    const row = q<HTMLButtonElement>('button[data-sprint="backlog"]')!;
    expect(row.textContent).toContain('Backlog');
    row.click();
    await settle();
    expect(row.getAttribute('aria-current')).toBe('true');
    expect(q('[data-strip]')!.textContent).toContain('1 ticket in no sprint');
    expect(q('[data-ticket="P-783"]')).not.toBe(null);
  });
});

describe('not connected', () => {
  it('points to Integrations', async () => {
    const { q } = await open({
      tryListSprints: async () => ({
        ok: false,
        error: { status: 404, message: 'not_connected', body: { error: 'not_connected' } },
      }),
    });
    const view = q('[data-view="not_connected"]')!;
    expect(view.textContent).toContain('This project has no tracker connected.');
    expect(view.querySelector('a')!.getAttribute('href')).toBe('/project/p-1/integrations');
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
    ['not connected', { tryListSprints: async () => ({ ok: false, error: { status: 404, message: 'not_connected', body: { error: 'not_connected' } } }) }, false],
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
