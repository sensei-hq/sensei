// @vitest-environment jsdom
//
// The Tickets route as a link from Delivery Metrics meets it (tracker-delivery-metrics
// D2, gate 3): `?board=&ticket=` opens that ticket, from that board's lane.
import { describe, it, expect, afterEach, vi } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import { page } from '$app/state';
import Page from './+page.svelte';
import { WORDS, projectSdlc, sprintSdlc, ticketSdlc, usage } from '$lib/tracker-fixtures.js';

/** The stub's `url` is typed with the app's route paths; a spec sets any. */
const setUrl = (href: string) => ((page as unknown as { url: URL }).url = new URL(href));

let cleanup: Array<() => void> = [];
afterEach(() => {
  cleanup.forEach((fn) => fn());
  cleanup = [];
  vi.unstubAllGlobals();
  setUrl('http://localhost/');
});

const board = (id: string, name: string) => ({
  id,
  kind: 'jira',
  base_url: 'https://acme.atlassian.net',
  account: 'a@b.c',
  external_scope: 'AUTH',
  board_ref: id,
  board_name: name,
  level: 'sprint',
  status: 'connected',
  last_success_at: null,
  last_error: null,
  counts: { tasks: 1, sprints: 1 },
  writeback: { enabled: false, state: 'disabled', surface: 'comment', mapped: 0, assigned: 0, failing: 0, error: null },
  sdlc_project: projectSdlc(),
});

const detail = (key: string) => ({
  task: { key, summary: 'Login', issue_type: 'Story', status: 'Done', removed: false, assigned_to_me: null, writeback_status: null, writeback_at: null },
  usage: usage(),
  sdlc: ticketSdlc(),
  by_repo: [],
  sessions: [],
});

function daemon() {
  const asked: string[] = [];
  const routes: Array<[RegExp, (m: RegExpMatchArray) => unknown]> = [
    [/\/trackers$/, () => [board('t-1', 'AUTH Scrum'), board('t-2', 'Mobile Scrum')]],
    [/\/api\/reason-codes\/(.+)$/, (m) => ({ domain: m[1], reasons: WORDS })],
    [/\/sprints$/, () => [{ id: 's1', name: 'Sprint 1', state: 'active', start_at: null, end_at: null, complete_at: null, tasks: 1, candidates_in_window: null, linked_sprint: null, linked_other: null, unlinked: null, reason: 'no_dates', sdlc: sprintSdlc() }]],
    [/\/backlog$/, () => ({ tickets: 0, usage: usage() })],
    [/\/tasks(\?.*)?$/, () => []],
    [/\/tracker-tasks\/([^/?]+)$/, (m) => detail(decodeURIComponent(m[1]))],
  ];
  vi.stubGlobal('fetch', async (url: string) => {
    const path = new URL(url).pathname + new URL(url).search;
    asked.push(path);
    for (const [re, body] of routes) {
      const m = path.match(re);
      if (m) return new Response(JSON.stringify(body(m)), { status: 200 });
    }
    return new Response('{}', { status: 404 });
  });
  return asked;
}

describe('a link from Delivery Metrics', () => {
  it('?board=t-2&ticket=AUTH-2 opens AUTH-2 from the Mobile Scrum lane', async () => {
    const asked = daemon();
    page.params.id = 'p-1';
    setUrl('http://localhost/workspace/p-1/tickets?board=t-2&ticket=AUTH-2');
    const m = mountComponent(Page, {});
    cleanup.push(m.destroy);
    await vi.waitFor(() => expect(m.container.querySelector('[data-view="ticket"]')).not.toBe(null));
    expect(asked).toContain('/api/workspaces/p-1/tracker-tasks/AUTH-2');
    expect(m.container.querySelector('[data-view="ticket"] h2')!.textContent).toContain('AUTH-2');
    expect(m.container.querySelector('[data-action="back"]')!.textContent).toBe('Back to Mobile Scrum · All tickets');
  });

  it('without a link, no ticket opens', async () => {
    const asked = daemon();
    page.params.id = 'p-1';
    setUrl('http://localhost/workspace/p-1/tickets');
    const m = mountComponent(Page, {});
    cleanup.push(m.destroy);
    await vi.waitFor(() => expect(m.container.querySelector('[data-lane="t-1"]')).not.toBe(null));
    expect(asked.some((p) => p.includes('/tracker-tasks/'))).toBe(false);
    expect(m.container.querySelector('[data-view="ticket"]')).toBe(null);
  });
});
