// @vitest-environment jsdom
//
// Delivery Metrics as a user meets it (tracker-delivery-metrics D2): each sprint's
// figures, "Your sessions" effort that never shows zeros for a sprint with no
// window, tickets that link to Tickets, and failures that never pass for "no board".
import { describe, it, expect, afterEach } from 'vitest';
import axe from 'axe-core';
import { mountComponent } from '$lib/test-mount.js';
import DeliveryScreen from './DeliveryScreen.svelte';
import { DeliveryState, type DeliveryApi } from './delivery-state.svelte.js';
import { WORDS, projectSdlc, sprintSdlc, ticketSdlc, usage } from '$lib/tracker-fixtures.js';
import type { SprintUsageRow, TrackerConnection } from '$lib/types.js';

let cleanup: Array<() => void> = [];
afterEach(() => {
  cleanup.forEach((fn) => fn());
  cleanup = [];
});

const settle = () => new Promise((r) => setTimeout(r, 0));

const tracker: TrackerConnection = {
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
};

const active: SprintUsageRow = {
  id: 's13',
  name: 'Sprint 13',
  state: 'active',
  start_at: '2026-10-01T12:00:00Z',
  end_at: '2026-10-15T12:00:00Z',
  complete_at: null,
  tasks: 2,
  candidates_in_window: 1,
  linked_sprint: usage({ sessions: 1, active_secs: 600, active_n: 1, api_cost: 1.24, api_cost_n: 1, metered_cost: 9, metered_cost_n: 1 }),
  linked_other: usage(),
  unlinked: usage(),
  reason: null,
  sdlc: sprintSdlc({
    commitment_reliability: 0,
    velocity: 0,
    facts: { committed: 4, added: 0, removed: 0, completed_committed: 0, completed_added: 0, carried_over: 0, unknown_status: 0 },
    reasons: { total_completion: 'history_pending' },
  }),
};

const future: SprintUsageRow = {
  ...active,
  id: 's14',
  name: 'Sprint 14',
  state: 'future',
  start_at: null,
  end_at: null,
  candidates_in_window: null,
  linked_sprint: null,
  linked_other: null,
  unlinked: null,
  reason: 'not_started',
  sdlc: sprintSdlc(),
};

function api(over: Partial<DeliveryApi> = {}): DeliveryApi {
  return {
    tryListTrackers: async () => ({ ok: true, data: [tracker] }),
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
          usage: usage(),
          sdlc: ticketSdlc(),
        },
      ],
    }),
    tryGetReasonCodes: async (domain) => ({ ok: true, data: { domain, reasons: WORDS } }),
    ...over,
  };
}

async function open(over: Partial<DeliveryApi> = {}) {
  const screen = new DeliveryState(api(over), 'p-1');
  const { container, destroy } = mountComponent(DeliveryScreen, { screen, workspaceId: 'p-1' });
  cleanup.push(destroy);
  await screen.load();
  await settle();
  const q = <T extends Element = Element>(sel: string) => container.querySelector<T>(sel);
  const qa = (sel: string) => [...container.querySelectorAll(sel)];
  return { container, screen, q, qa };
}

describe('Delivery Metrics', () => {
  it("shows the board's project row and one row per sprint in the daemon's order", async () => {
    const { q, qa } = await open();
    expect(q('[data-delivery-project] [data-sdlc="predictability"]')!.textContent).toBe('Too few sprints');
    expect(qa('[data-sprint]').map((r) => r.getAttribute('data-sprint'))).toEqual(['s14', 's13']);
  });

  it('predictability is the board’s; tokens and prompts per ticket are labelled as your sessions', async () => {
    const { q } = await open();
    const owner = (key: string) => q(`[data-sdlc="${key}"]`)!.closest('[data-delivery-project]')!;
    expect(owner('predictability').getAttribute('data-delivery-project')).toBe('board');
    expect(owner('predictability').textContent).not.toContain('Your sessions');
    for (const key of ['tokens_per_ticket', 'prompts_per_ticket']) {
      expect(owner(key).getAttribute('data-delivery-project')).toBe('yours');
      expect(owner(key).textContent).toContain('Your sessions');
    }
  });

  it('a sprint row shows its seven figures; a real 0 as 0, a null as its reason', async () => {
    const { q } = await open();
    const row = q('[data-sprint="s13"]')!;
    expect(row.querySelector('[data-sdlc="commitment_reliability"]')!.textContent).toBe('0%');
    expect(row.querySelector('[data-sdlc="velocity"]')!.textContent).toBe('0 pts');
    expect(row.querySelector('[data-sdlc="total_completion"]')!.textContent).toBe('Ticket history not read yet');
    expect(row.querySelectorAll('[data-sdlc]').length).toBe(7);
  });

  it('effort is headed "Your sessions" and its cost is the API cost, not the metered cost', async () => {
    const { container, q } = await open();
    expect(container.textContent).toContain('Your sessions');
    const row = q('[data-sprint="s13"]')!;
    expect(row.querySelector('[data-effort="active"]')!.textContent).toBe('10m');
    expect(row.querySelector('[data-effort="cost"]')!.textContent).toBe('$1.24');
  });

  it('a sprint that has not started shows why in its effort cells, never 0', async () => {
    const { q } = await open();
    const effort = q('[data-sprint="s14"] [data-effort="reason"]')!;
    expect(effort.textContent).toBe('Not started. Usage counts from the sprint’s start.');
    expect(q('[data-sprint="s14"] [data-effort="active"]')).toBe(null);
  });

  it('selecting a sprint shows its counts and its tickets, each linking to Tickets', async () => {
    const { q, screen } = await open();
    q<HTMLButtonElement>('[data-sprint="s13"] [data-action="select"]')!.click();
    await settle();
    await settle();
    expect(screen.boards[0].selected?.id).toBe('s13');
    expect(q('[data-fact="committed"]')!.textContent).toBe('4');
    const link = q<HTMLAnchorElement>('[data-ticket="AUTH-2"]')!;
    expect(link.getAttribute('href')).toBe('/workspace/p-1/tickets?board=t&ticket=AUTH-2');
    expect(link.textContent).toContain('Login');
  });

  it('a sprint with no replay says why; unknown statuses are named', async () => {
    const { q, screen } = await open({
      tryListSprints: async () => ({
        ok: true,
        data: [future, { ...active, sdlc: { ...active.sdlc, facts: { ...active.sdlc.facts!, unknown_status: 2 } } }],
      }),
    });
    await screen.boards[0].select('s14');
    await settle();
    expect(q('[data-selected="s14"]')!.textContent).toContain('Ticket history not read yet');
    expect(q('[data-fact]')).toBe(null);
    await screen.boards[0].select('s13');
    await settle();
    expect(q('[data-selected="s13"]')!.textContent).toContain('2 tickets with an unknown status');
  });

  it("a failed ticket read is an alert, and a sprint with no tickets says so — never a blank panel", async () => {
    const failing = await open({ tryListTrackerTasks: async () => ({ ok: false, error: { status: 500, message: 'Internal' } }) });
    await failing.screen.boards[0].select('s13');
    await settle();
    expect(failing.q('[data-selected="s13"] [role="alert"]')).not.toBe(null);
    const empty = await open({ tryListTrackerTasks: async () => ({ ok: true, data: [] }) });
    await empty.screen.boards[0].select('s13');
    await settle();
    expect(empty.q('[data-selected="s13"]')!.textContent).toContain('This sprint lists no tickets.');
  });

  it('shows no ticket SDLC figure (B2)', async () => {
    const { q, container } = await open();
    q<HTMLButtonElement>('[data-sprint="s13"] [data-action="select"]')!.click();
    await settle();
    for (const key of ['effort_cost', 'compile_defects', 'execution_hours', 'rework_minutes']) {
      expect(container.querySelector(`[data-sdlc="${key}"]`), key).toBe(null);
    }
  });

  it('no sprint board shows the not-connected words and the Integrations link', async () => {
    const { q } = await open({ tryListTrackers: async () => ({ ok: true, data: [] }) });
    expect(q('[data-view="not_connected"]')!.textContent).toContain('This workspace has no sprint board connected.');
    expect(q('[data-view="not_connected"] a')!.getAttribute('href')).toBe('/workspace/p-1/integrations');
  });

  it('a 500 from /sprints is the daemon error with Retry, which recovers', async () => {
    let calls = 0;
    const { q } = await open({
      tryListSprints: async () =>
        ++calls === 1 ? { ok: false, error: { status: 500, message: 'Internal' } } : { ok: true, data: [active] },
    });
    expect(q('[data-view="daemon_error"]')).not.toBe(null);
    expect(q('[data-sprint]')).toBe(null);
    q<HTMLButtonElement>('[data-action="retry"]')!.click();
    await settle();
    await settle();
    expect(q('[data-view="daemon_error"]')).toBe(null);
    expect(q('[data-sprint="s13"]')).not.toBe(null);
  });

  it('has no serious or critical axe violations', async () => {
    const { container } = await open();
    const res = await axe.run(container, {
      runOnly: { type: 'tag', values: ['wcag2a', 'wcag2aa'] },
      rules: { 'color-contrast': { enabled: false } },
    });
    expect(res.violations.filter((v) => v.impact === 'serious' || v.impact === 'critical')).toEqual([]);
  });
});
