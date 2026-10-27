// @vitest-environment jsdom
//
// The Integrations screen as a user meets it: labelled fields, a status region
// that is announced, failures that take focus, a confirming dialog, and a token
// that never lingers in the page.
import { describe, it, expect, afterEach } from 'vitest';
import axe from 'axe-core';
import { flushSync } from 'svelte';
import { mountComponent } from '$lib/test-mount.js';
import IntegrationsScreen from './IntegrationsScreen.svelte';
import { TrackerIntegration, type TrackerApi } from './integrations-state.svelte.js';
import type { TrackerConnection } from '$lib/types.js';

const SENTINEL = 'SENTINEL-TOKEN-XYZ';

let cleanup: Array<() => void> = [];
afterEach(() => {
  cleanup.forEach((fn) => fn());
  cleanup = [];
});

const connected: TrackerConnection = {
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
};

function api(over: Partial<TrackerApi> = {}): TrackerApi {
  return {
    tryListTrackers: async () => ({ ok: true, data: [] }),
    tryConnectTracker: async () => ({
      ok: false,
      error: { status: 401, message: 'unauthorized', body: { error: 'unauthorized' } },
    }),
    trySyncTracker: async () => ({
      ok: true,
      data: { counts: { sprints: 6, tasks: 142, removed: 0, links: 0, skipped_members: 0 } },
    }),
    tryDisconnectTracker: async () => ({ ok: true, data: undefined }),
    trySetTrackerWriteback: async () => ({ ok: true, data: connected }),
    ...over,
  };
}

async function open(over: Partial<TrackerApi> = {}) {
  const state = new TrackerIntegration(api(over), 'p-1', {
    now: () => new Date('2026-10-18T12:00:00Z'),
    schedule: () => () => {},
  });
  await state.load();
  const m = mountComponent(IntegrationsScreen, { screen: state });
  cleanup.push(m.destroy);
  flushSync();
  const q = <T extends Element = HTMLElement>(sel: string) => m.container.querySelector(sel) as T | null;
  return { state, m, q };
}

const settle = async () => {
  await new Promise((r) => setTimeout(r, 0));
  flushSync();
};

function labelOf(input: HTMLElement, container: HTMLElement): string {
  return container.querySelector(`label[for="${input.id}"]`)?.textContent?.trim() ?? '';
}

describe('not connected', () => {
  it('labels every field and hides the token', async () => {
    const { m, q } = await open();
    const board = q('[data-field="board_url"]') as HTMLInputElement;
    const email = q('[data-field="email"]') as HTMLInputElement;
    const token = q('[data-field="token"]') as HTMLInputElement;
    expect(labelOf(board, m.container)).toBe('Board URL');
    expect(labelOf(email, m.container)).toBe('Atlassian account email');
    expect(labelOf(token, m.container)).toBe('API token');
    expect(token.type).toBe('password');
    expect(email.type).toBe('email');
  });

  it('offers Jira and shows GitLab as coming soon, not selectable', async () => {
    const { q } = await open();
    const gitlab = q('[data-tracker="gitlab"]')!;
    expect(gitlab.getAttribute('aria-disabled')).toBe('true');
    expect(gitlab.textContent).toMatch(/coming soon/i);
  });

  it('links to Atlassian to create a token and says where it is kept', async () => {
    const { m, q } = await open();
    const link = q<HTMLAnchorElement>('a[data-link="create-token"]')!;
    expect(link.href).toBe('https://id.atlassian.com/manage-profile/security/api-tokens');
    expect(m.container.textContent).toContain(
      'Your token is stored in your system keychain, not in sensei’s database.',
    );
  });

  it('keeps Connect disabled until the form is complete', async () => {
    const { state, q } = await open();
    const button = () => q<HTMLButtonElement>('[data-action="connect"]')!;
    expect(button().disabled).toBe(true);
    state.boardUrl = 'https://acme.atlassian.net/jira/software/projects/AUTH/boards/42';
    state.email = 'dev@example.com';
    state.token = 'tok';
    flushSync();
    expect(button().disabled).toBe(true);
    q<HTMLInputElement>('input[name="level"][value="sprint"]')!.click();
    flushSync();
    expect(state.level).toBe('sprint');
    expect(button().disabled).toBe(false);
  });

  it('asks what the board serves, with nothing chosen for the user', async () => {
    const { m, q } = await open();
    const radios = [...m.container.querySelectorAll<HTMLInputElement>('input[name="level"]')];
    expect(radios.map((r) => r.value)).toEqual(['epic', 'sprint', 'task']);
    expect(radios.some((r) => r.checked)).toBe(false);
    expect(q('[data-field="level"] legend')?.textContent).toBe('What the board serves');
  });

  it('a rejected token: the alert takes focus, names the token field, and the token is gone', async () => {
    const { state, m, q } = await open();
    state.level = 'sprint';
    state.boardUrl = 'https://acme.atlassian.net/jira/software/projects/AUTH/boards/42';
    state.email = 'dev@example.com';
    state.token = SENTINEL;
    flushSync();
    q<HTMLButtonElement>('[data-action="connect"]')!.click();
    await settle();
    const alert = q('[role="alert"]')!;
    expect(alert.textContent).toContain('Jira rejected the email and token.');
    expect(document.activeElement).toBe(alert);
    const token = q<HTMLInputElement>('[data-field="token"]')!;
    expect(token.getAttribute('aria-describedby')).toBe(alert.id);
    expect(token.value).toBe('');
    expect(m.container.innerHTML).not.toContain(SENTINEL);
    expect(JSON.stringify({ ...localStorage })).not.toContain(SENTINEL);
    expect(JSON.stringify(state)).not.toContain(SENTINEL);
  });

  it('announces progress in a status region', async () => {
    const { q } = await open();
    expect(q('[role="status"]')).not.toBeNull();
  });
});

describe('daemon error', () => {
  it('shows the failure and Retry, never the connect form', async () => {
    const { q } = await open({
      tryListTrackers: async () => ({ ok: false, error: { status: 0, message: 'refused' } }),
    });
    expect(q('[data-view="daemon_error"]')?.textContent).toContain('The sensei daemon didn’t respond.');
    expect(q('[data-action="retry"]')).not.toBeNull();
    expect(q('[data-field="board_url"]')).toBeNull();
  });
});

describe('connected', () => {
  it('shows the card with its status as text and its actions', async () => {
    const { q } = await open({ tryListTrackers: async () => ({ ok: true, data: [connected] }) });
    const card = q('[data-tracker="t-1"]')!;
    expect(card.textContent).toContain('Jira · acme.atlassian.net · project AUTH · board "AUTH Scrum" (#42)');
    expect(card.textContent).toContain('connected · synced 2 min ago · 6 sprints · 142 tickets');
    for (const a of ['sync', 'replace', 'disconnect']) {
      expect(card.querySelector(`[data-action="${a}"]`)).not.toBeNull();
    }
    expect(q('[data-field="board_url"]')).toBeNull();
  });

  it('lists the boards under their level, epics first, each with its own actions', async () => {
    const epic = { ...connected, id: 't-2', level: 'epic' as const, board_ref: '43', board_name: 'Portfolio' };
    const synced: string[] = [];
    const { m, q } = await open({
      tryListTrackers: async () => ({ ok: true, data: [connected, epic] }),
      trySyncTracker: async (_p, t) => {
        synced.push(t);
        return { ok: true, data: { counts: { sprints: 0, tasks: 0, removed: 0, links: 0, skipped_members: 0 } } };
      },
    });
    const groups = [...m.container.querySelectorAll<HTMLElement>('[data-level]')];
    expect(groups.map((g) => [g.dataset.level, g.querySelector('h2')?.textContent])).toEqual([
      ['epic', 'Epics'],
      ['sprint', 'Sprints'],
    ]);
    q('[data-tracker="t-2"] [data-action="sync"]')!.click();
    await settle();
    expect(synced).toEqual(['t-2']);
  });

  it('Add a board opens the form beside the boards; Cancel closes it', async () => {
    const { q } = await open({ tryListTrackers: async () => ({ ok: true, data: [connected] }) });
    q<HTMLButtonElement>('[data-action="add"]')!.click();
    flushSync();
    expect(q('[data-field="board_url"]')).not.toBeNull();
    expect(q('[data-tracker="t-1"]')).not.toBeNull();
    q<HTMLButtonElement>('[data-action="cancel-add"]')!.click();
    flushSync();
    expect(q('[data-field="board_url"]')).toBeNull();
  });

  it('Disconnect opens a dialog with Cancel focused; Cancel closes it', async () => {
    const { q } = await open({ tryListTrackers: async () => ({ ok: true, data: [connected] }) });
    q<HTMLButtonElement>('[data-action="disconnect"]')!.click();
    flushSync();
    const dialog = q('[data-component="disconnect-dialog"]')!;
    expect(dialog.textContent).toContain('Disconnect board #42?');
    expect(dialog.textContent).toContain(
      'This removes this board’s synced sprints, tickets and session links from sensei. The workspace’s other boards stay, and Jira itself is not changed.',
    );
    expect((document.activeElement as HTMLElement).dataset.action).toBe('cancel-disconnect');
    (document.activeElement as HTMLButtonElement).click();
    flushSync();
    expect(q('[data-component="disconnect-dialog"]')).toBeNull();
  });

  it('confirming returns to the connect form', async () => {
    let listed = [connected];
    const { q } = await open({
      tryListTrackers: async () => ({ ok: true, data: listed }),
      tryDisconnectTracker: async () => {
        listed = [];
        return { ok: true, data: undefined };
      },
    });
    q<HTMLButtonElement>('[data-action="disconnect"]')!.click();
    flushSync();
    q<HTMLButtonElement>('[data-action="confirm-disconnect"]')!.click();
    await settle();
    expect(q('[data-field="board_url"]')).not.toBeNull();
  });
});

describe('accessibility (axe, structural rules; contrast is checked in the e2e scan)', () => {
  const states: Array<[string, Partial<TrackerApi>]> = [
    ['not connected', {}],
    ['connected', { tryListTrackers: async () => ({ ok: true, data: [connected] }) }],
    ['daemon error', { tryListTrackers: async () => ({ ok: false, error: { status: 0, message: 'x' } }) }],
  ];
  for (const [name, over] of states) {
    it(`${name}: no serious or critical violations`, async () => {
      const { m } = await open(over);
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

describe('write-back on the board card (W-UI)', () => {
  const active = { ...connected, writeback: { ...connected.writeback, state: 'active' as const, assigned: 2 } };

  it('the header says what sensei writes back, and where', async () => {
    const { m } = await open({ tryListTrackers: async () => ({ ok: true, data: [active] }) });
    expect(m.container.textContent).toContain(
      'Connect this workspace’s Jira boards, each at a level, so sensei can link AI sessions to tickets and report time, tokens and cost per ticket. On tickets assigned to you, sensei writes these numbers back to Jira, as a comment or in the fields you map.',
    );
    expect(m.container.textContent).not.toContain('never changes anything in Jira');
  });

  it('each card shows its write-back state and a labelled switch that sends one request', async () => {
    const calls: Array<boolean> = [];
    const { q } = await open({
      tryListTrackers: async () => ({ ok: true, data: [active] }),
      trySetTrackerWriteback: async (_p, _t, enabled) => {
        calls.push(enabled);
        return { ok: true, data: { ...active, writeback: { ...active.writeback, enabled, state: 'disabled' } } };
      },
    });
    const line = q('[data-tracker="t-1"] [data-writeback-state]');
    expect(line?.getAttribute('data-writeback-state')).toBe('active');
    expect(line?.textContent?.trim()).toBe('Writing to 2 tickets assigned to you.');
    const toggle = q<HTMLInputElement>('[data-tracker="t-1"] input[data-action="writeback"]');
    expect(toggle?.type).toBe('checkbox');
    expect(toggle?.checked).toBe(true);
    expect(toggle?.labels?.[0]?.textContent).toContain('Write back to Jira');
    toggle!.click();
    await settle();
    expect(calls).toEqual([false]);
    expect(q('[data-tracker="t-1"] [data-writeback-state]')?.getAttribute('data-writeback-state')).toBe('disabled');
    expect(q<HTMLInputElement>('[data-tracker="t-1"] input[data-action="writeback"]')?.checked).toBe(false);
  });

  it.each([
    ['active', { assigned: 2 }, 'Writing to 2 tickets assigned to you.'],
    ['disabled', { enabled: false }, 'Write-back is off.'],
    ['pass_failed', { error: 'db' }, 'Couldn’t record what was written. Write-back retries on each sync.'],
    [
      'owner_not_a_person',
      {},
      'This token belongs to an app account, so sensei can’t tell which tickets are yours. Nothing is written.',
    ],
    [
      'owner_lookup_failed',
      { error: 'forbidden' },
      'Couldn’t check which tickets are yours: This account can’t read that board. Retrying on each sync.',
    ],
    ['owner_unknown', {}, 'Checking which tickets are yours on the next sync.'],
    ['nothing_assigned', { assigned: 0 }, 'No tickets on this board are assigned to you.'],
  ] as const)('the card says %s in words', async (state, over, text) => {
    const card = { ...connected, writeback: { ...connected.writeback, state, ...over } };
    const { q } = await open({ tryListTrackers: async () => ({ ok: true, data: [card] }) });
    const line = q('[data-tracker="t-1"] [data-writeback-state]');
    expect(line?.getAttribute('data-writeback-state')).toBe(state);
    expect(line?.textContent?.trim()).toBe(text);
  });

  it('the switch is disabled while a request runs and keeps its place until the daemon answers', async () => {
    let answer: (r: { ok: true; data: TrackerConnection }) => void = () => {};
    const { q } = await open({
      tryListTrackers: async () => ({ ok: true, data: [active] }),
      trySetTrackerWriteback: () => new Promise((r) => (answer = r)),
    });
    const toggle = () => q<HTMLInputElement>('[data-tracker="t-1"] input[data-action="writeback"]')!;
    toggle().click();
    flushSync();
    expect(toggle().checked).toBe(true);
    expect(toggle().disabled).toBe(true);
    answer({ ok: true, data: { ...active, writeback: { ...active.writeback, enabled: false, state: 'disabled' } } });
    await settle();
    expect(toggle().checked).toBe(false);
    expect(toggle().disabled).toBe(false);
  });

  it('a refused switch flips back and the alert says why', async () => {
    const { q } = await open({
      tryListTrackers: async () => ({ ok: true, data: [active] }),
      trySetTrackerWriteback: async () => ({
        ok: false,
        error: { status: 403, message: 'forbidden_caller', body: { error: 'forbidden_caller' } },
      }),
    });
    q<HTMLInputElement>('[data-tracker="t-1"] input[data-action="writeback"]')!.click();
    await settle();
    expect(q<HTMLInputElement>('[data-tracker="t-1"] input[data-action="writeback"]')?.checked).toBe(true);
    expect(q('[role="alert"]')?.textContent).toContain('refused this window’s request');
  });
});
