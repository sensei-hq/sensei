// @vitest-environment jsdom
// Workspace · Relay — the screen that replaces Intake (D90, relay slice 1 item 4).
//
// Contract: coord/mockups/relay-lanes/CONTRACT.md §1 (the screen), §3 (the shapes);
// data: docs/prd/modules/03-delivery/21-relay/lanes-data.md (`relay.now`, the queue).
//
// The loader and the page are imported inside each test, so a broken route fails one
// test at a time rather than the whole file and the live specs beside it.
//
// The pattern is activity-logs' (D87 adopter): the loader reads, the page draws `data`
// and reads nothing of its own, a state class owns each Retry (recovers, fails again
// with the new reason, one read per press), and rokkit's ScreenShell draws the
// layout. The daemon's relay API does not exist yet, so its client
// (`$lib/relay-api`, `relayApi(port)`) is mocked: every read and write is a spy.
import { describe, it, expect, afterEach, vi } from 'vitest';
import { tick } from 'svelte';
import { mountComponent } from '$lib/test-mount.js';
import { KANJI_ICONS } from '$lib/glyph-vocabulary';
import type { ApiResult } from '$lib/api.js';
import type { RelayNow, RelayQueue } from '$lib/relay-api.js';
import * as F from '$lib/test-stubs/relay-fixtures.js';

// Variables, so the transform does not resolve them before phase B creates them.
const PAGE = './+page.svelte';
const LOADER = './+page.js';

// The relay socket is a stream the spec drives: opened, and fired, by hand (#421).
const relay = vi.hoisted(() => {
  type Stream = {
    onopen: (() => void) | null;
    handler: ((e: unknown) => void) | null;
    destroyed: boolean;
    subscribe(h: (e: unknown) => void): () => void;
    destroy(): void;
    open(): void;
    fire(e: unknown): void;
  };
  const streams: Stream[] = [];
  const stream = (): Stream => {
    const s: Stream = {
      onopen: null,
      handler: null,
      destroyed: false,
      subscribe(h) {
        s.handler = h;
        return () => (s.handler = null);
      },
      destroy() {
        s.destroyed = true;
      },
      open() {
        s.onopen?.();
      },
      fire(e) {
        s.handler?.(e);
      },
    };
    streams.push(s);
    return s;
  };
  return { api: null as unknown, streams, stream };
});
vi.mock('$lib/relay-api.js', () => ({ relayApi: vi.fn(() => relay.api), relayStream: vi.fn(() => relay.stream()) }));

const route = vi.hoisted(() => ({ id: 'w1' }));
vi.mock('$app/state', () => ({
  page: {
    get params() {
      return { id: route.id };
    },
    get url() {
      return new URL(`http://localhost/workspace/${route.id}/relay`);
    },
  },
}));
vi.mock('$app/navigation', () => ({ goto: vi.fn(), invalidateAll: vi.fn() }));

// D87: the DOM hooks alone can't tell rokkit's shell from a lookalike, so the shell
// itself is spied on and asked for this view's id.
vi.mock('@rokkit/app', async (importOriginal) => {
  const m = (await importOriginal()) as Record<string, unknown>;
  return { ...m, ScreenShell: vi.fn(m.ScreenShell as never) };
});

// The shell and the lane marks follow the reader's glyph mode; only the mode is stubbed.
const glyphs = vi.hoisted(() => ({ mode: 'kanji' as 'kanji' | 'icon' }));
vi.mock('$lib/glyphs', async (importOriginal) => {
  const m = (await importOriginal()) as Record<string, unknown>;
  return { ...m, useGlyphMode: () => () => glyphs.mode };
});

import { ScreenShell } from '@rokkit/app';

let cleanup: Array<() => void> = [];
afterEach(() => {
  cleanup.forEach((fn) => fn());
  cleanup = [];
  relay.streams.length = 0;
  vi.unstubAllGlobals();
  vi.mocked(ScreenShell).mockClear();
  glyphs.mode = 'kanji';
  route.id = 'w1';
});

type Opened = { c: HTMLElement; api: F.FakeRelayApi; data: unknown };

/** The loader's read for this workspace, then the page drawn from it. */
async function loadFor(workspace: string, api: F.FakeRelayApi) {
  relay.api = api;
  route.id = workspace;
  const { load } = await import(/* @vite-ignore */ LOADER);
  return load({
    params: { id: workspace },
    url: new URL(`http://localhost/workspace/${workspace}/relay`),
  } as never);
}

async function open(
  opts: {
    now?: ApiResult<RelayNow>;
    queue?: ApiResult<RelayQueue>;
    workspace?: string;
    /** The workspace layout's data, which SvelteKit merges into the page's `data`. */
    layout?: Record<string, unknown>;
  } = {},
): Promise<Opened> {
  const api = F.fakeRelayApi();
  if (opts.now) api.tryGetNow.mockResolvedValue(opts.now);
  if (opts.queue) api.tryGetQueue.mockResolvedValue(opts.queue);
  const loaded = await loadFor(opts.workspace ?? 'w1', api);
  const data = { ...(opts.layout ?? {}), ...(loaded as object) };
  const { default: Page } = await import(/* @vite-ignore */ PAGE);
  const m = mountComponent(Page, { data } as never);
  cleanup.push(m.destroy);
  await tick();
  return { c: m.container, api, data };
}

const one = (root: ParentNode, sel: string) => root.querySelector(sel) as HTMLElement | null;
const all = (root: ParentNode, sel: string) => [...root.querySelectorAll(sel)] as HTMLElement[];
const text = (el: Element | null | undefined) => (el?.textContent ?? '').replace(/\s+/g, ' ').trim();

/** The element, or a failure that names what is missing — never a null dereference,
 *  so a red says which part of the screen is not there yet. */
function need<T extends Element>(el: T | null | undefined, what: string): T {
  expect(el ?? null, `missing: ${what}`).not.toBeNull();
  return el as T;
}
const section = (c: HTMLElement, name: string) => one(c, `[data-relay-section="${name}"]`);

function triageRows(c: HTMLElement) {
  const rows = all(need(section(c, 'triage'), 'the triage section'), '[data-queue-row]');
  expect(rows.length, 'triage rows').toBeGreaterThan(0);
  return rows;
}

/** A header view button by its name ("Now", "Queue"), wherever the header draws it. The
 *  mockup's buttons are icons: the name is the accessible name, with the count ("Queue · 7"). */
function tab(c: HTMLElement, label: string): HTMLElement | null {
  const header = one(c, '[data-screen-header]');
  if (!header) return null;
  return (
    all(header, '[role="tab"], button, a').find((el) =>
      (el.getAttribute('aria-label') || text(el)).startsWith(label),
    ) ?? null
  );
}
const isActive = (el: HTMLElement | null) =>
  el?.getAttribute('aria-selected') === 'true' || el?.getAttribute('aria-pressed') === 'true';

function toolbarButton(c: HTMLElement, label: string) {
  return one(c, `[data-screen-tools] [aria-label="${label}"]`) as HTMLButtonElement | null;
}

function setValue(el: HTMLElement | null, value: string, event = 'input') {
  const field = el as HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement;
  field.value = value;
  field.dispatchEvent(new Event(event, { bubbles: true }));
}

const shellProps = () => vi.mocked(ScreenShell).mock.calls[0][1] as unknown as Record<string, unknown>;

// ─── The loader's read ──────────────────────────────────────────────────────

describe('Relay — the loader’s read', () => {
  it('reads this workspace’s crew and queue, and the page reads nothing of its own', async () => {
    const fetchSpy = vi.fn();
    vi.stubGlobal('fetch', fetchSpy);
    const { api } = await open({ workspace: 'w2' });
    expect(api.tryGetNow).toHaveBeenCalledTimes(1);
    // The workspace window is the same screen pre-filtered to its workspace (repository ruling).
    expect(api.tryGetNow).toHaveBeenCalledWith({ workspace_id: 'w2' });
    expect(api.tryGetQueue).toHaveBeenCalledTimes(1);
    expect(api.tryGetQueue).toHaveBeenCalledWith({ workspace_id: 'w2' });
    expect(api.tryGetNow).not.toHaveBeenCalledWith({ workspace_id: 'w1' });
    expect(fetchSpy).not.toHaveBeenCalled();
  });
});

// ─── Now · Needs you ────────────────────────────────────────────────────────

// #421: the page builds the relay state over the daemon's source and connects it; the
// socket's events apply to the state, so a row that arrives redraws that row alone.
describe('Relay — the live stream (#421)', () => {
  const settle = () => new Promise((r) => setTimeout(r, 0));

  it('the page connects one socket; its first open reads Now and the queue (nothing before it is missed)', async () => {
    const { api } = await open();
    expect(relay.streams).toHaveLength(1);
    api.tryGetNow.mockClear();
    api.tryGetQueue.mockClear();
    relay.streams[0].open();
    await settle();
    expect(api.tryGetNow).toHaveBeenCalledWith({ workspace_id: 'w1' });
    expect(api.tryGetQueue).toHaveBeenCalledWith({ workspace_id: 'w1' });
  });

  it('a lane row that arrives changes that lane on the screen, with no read', async () => {
    const { c, api } = await open();
    const lane = F.now().lanes[1];
    const reads = api.tryGetNow.mock.calls.length;
    relay.streams[0].fire({ action: 'update', entity: 'lane', data: { id: lane.id, title: 'Streamed objective' } });
    await tick();
    expect(text(need(one(c, `[data-lane-row="${lane.code}"]`), 'the lane row'))).toContain('Streamed objective');
    expect(api.tryGetNow.mock.calls.length).toBe(reads);
  });

  it('an event that names ids only (today’s daemon) reads Now again', async () => {
    const { api } = await open();
    const reads = api.tryGetNow.mock.calls.length;
    relay.streams[0].fire({ action: 'update', entity: 'task', data: { task_id: 't-structure' } });
    await settle();
    expect(api.tryGetNow.mock.calls.length).toBe(reads + 1);
  });

  it('leaving the screen closes the socket', async () => {
    await open();
    cleanup.forEach((fn) => fn());
    cleanup = [];
    expect(relay.streams[0].destroyed).toBe(true);
  });
});

describe('Relay · Now — Needs you', () => {
  it('is absent when nothing needs the person', async () => {
    // r2 L6: the default crew has triage (as the default queue does), so this one has none.
    const { c } = await open({ now: F.ok(F.now({ triage: [], decisions: [] })), queue: F.ok(F.queue({ triage: [] })) });
    expect(section(c, 'lanes')).not.toBeNull();
    expect(section(c, 'needs-you')).toBeNull();
  });

  it('a crew-to-person message: sending lane code, excerpt, and its task link', async () => {
    // Q2 part 2: to.person mail surfaces in Needs you newest-first. Read
    // state is the daemon's (delivered/read receipts retire the row); the
    // screen re-reads rather than guessing, as with answered decisions.
    const { c } = await open({
      now: F.ok(F.now({ decisions: [], triage: [], person_messages: [F.personMessage()] })),
    });
    const needs = need(section(c, 'needs-you'), 'needs-you');
    const row = need(one(needs, '[data-person-message]'), 'message row');
    expect(text(row)).toContain('L2');
    expect(text(row)).toContain('The migration needs your eyes');
    expect(one(row, `a[href="/workspace/w1/relay/${F.CREW_ID}/t-logs"]`)).not.toBeNull();
    // Retirement is the daemon's (read receipts); the screen draws what the
    // feed serves — an empty feed draws no section, per the test above.
  });

  it('each decision: question, context, options with its own recommended one marked, Answer, and its task', async () => {
    const { c } = await open({
      now: F.ok(
        F.now({
          decisions: [
            F.decision(),
            F.decision({
              id: 'ask-2',
              task_id: 't-billing',
              task_title: 'Billing build',
              question: 'Which plan ships first?',
              context: 'Billing is waiting on this.',
              options: [
                { id: 'team', label: 'Team', detail: null },
                { id: 'org', label: 'Organisation', detail: null },
              ],
              recommended: 'org',
            }),
          ],
        }),
      ),
    });
    const needs = need(section(c, 'needs-you'), 'the needs-you section');
    expect(needs).not.toBeNull();
    const rows = all(needs, '[data-decision]');
    expect(rows).toHaveLength(2);

    const [first, second] = rows;
    expect(text(first)).toContain('Where does the activity log live?');
    expect(text(first)).toContain('Activity logs is waiting on this.');
    expect(all(first, '[data-option]').map((o) => o.getAttribute('data-option'))).toEqual(['db', 'file']);
    expect(all(first, '[data-option][data-recommended]').map((o) => o.getAttribute('data-option'))).toEqual(['db']);
    expect(one(first, '[data-decision-answer]')).not.toBeNull();
    expect(one(first, `a[href="/workspace/w1/relay/${F.CREW_ID}/t-logs"]`)).not.toBeNull();

    // The recommended option is each decision's own, not "the first one".
    expect(all(second, '[data-option][data-recommended]').map((o) => o.getAttribute('data-option'))).toEqual(['org']);
    expect(one(second, `a[href="/workspace/w1/relay/${F.CREW_ID}/t-billing"]`)).not.toBeNull();
  });

  it('"N to triage" counts the queue’s triage items by source and opens the Queue, which shows them (the #379 brief)', async () => {
    const { c } = await open({ now: F.ok(F.now({ triage: [] })) });
    const line = need(one(c, '[data-triage-line]'), 'the triage line');
    expect(text(line)).toContain('2 to triage');
    expect(text(line)).toContain('From Jira sprint 47 and an open item');
    line.click();
    await tick();
    expect(section(c, 'triage')).not.toBeNull();
    expect(section(c, 'lanes')).toBeNull();
    expect(isActive(tab(c, 'Queue'))).toBe(true);
  });

  it('tasks no lane has taken are their own read-only row, apart from triage (the #379 brief)', async () => {
    const { c } = await open({
      now: F.ok(F.now({ triage: [{ provider: 'github', sprint: 'milestone 11', count: 3 }, { provider: 'manual', sprint: null, count: 1 }] })),
      queue: F.ok(F.queue({ triage: [] })),
    });
    const row = need(one(c, '[data-unassigned]'), 'the unassigned row');
    expect(text(row)).toContain('Unassigned · 4');
    expect(text(row)).toContain('From GitHub milestone 11 and an open item');
    expect(one(c, '[data-triage-line]')).toBeNull();
  });

  it('a wait at the threshold is in Needs you’s long-waits card, oldest first', async () => {
    const { c } = await open({
      now: F.ok(F.now({ lanes: [F.lane({ task: { ...F.lane().task!, state: 'approved', state_changed_at: F.minutesAgo(52) } })] })),
    });
    const card = need(one(c, '[data-relay-section="needs-you"] [data-stale-waits]'), 'the long-waits card');
    expect(text(one(card, '[data-stale-title]'))).toContain('1 task has waited 15 min or more');
    expect(all(card, '[data-wait-row]').map((r) => r.getAttribute('data-wait-row'))).toEqual(['t-structure']);
  });

  it('Answer sends the option the person picked; the event it causes takes the decision out of Needs you', async () => {
    const { c, api } = await open({ now: F.ok(F.now({ decisions: [F.decision()] })) });
    api.tryGetNow.mockResolvedValue(F.ok(F.now({ decisions: [] })));
    const reads = api.tryGetNow.mock.calls.length;

    const row = need(one(c, '[data-decision]'), '[data-decision]');
    need(one(row, '[data-option="file"]'), '[data-option="file"]').click();
    await tick();
    need(one(row, '[data-decision-answer]'), '[data-decision-answer]').click();

    await vi.waitFor(() => expect(api.tryAnswerDecision).toHaveBeenCalledTimes(1));
    expect(api.tryAnswerDecision).toHaveBeenCalledWith({
      decision_id: 'ask-1',
      option: 'file',
      note: null,
      surface: 'desktop',
    });
    // The answer is the daemon's to record: the screen does not remove the row itself,
    // nor read again on its own — the event the answer causes does (#421).
    expect(api.tryGetNow.mock.calls.length).toBe(reads);
    expect(one(c, '[data-decision]')).not.toBeNull();
    relay.streams[0].fire({ action: 'update', entity: 'task', data: { op: 'answer', task_id: 't-structure' } });
    await vi.waitFor(() => expect(one(c, '[data-decision]')).toBeNull());
  });

  it('a refused answer says why, keeps the decision, and can be sent again', async () => {
    const { c, api } = await open({ now: F.ok(F.now({ decisions: [F.decision()] })) });
    api.tryAnswerDecision.mockResolvedValue(F.fail(409, 'already answered on the dōjō'));

    const row = need(one(c, '[data-decision]'), '[data-decision]');
    need(one(row, '[data-option="db"]'), '[data-option="db"]').click();
    await tick();
    const answer = one(row, '[data-decision-answer]') as HTMLButtonElement;
    answer.click();

    await vi.waitFor(() => expect(text(one(c, '[data-decision-error]'))).toContain('already answered on the dōjō'));
    expect(one(c, '[data-decision]')).not.toBeNull();
    expect((need(one(c, '[data-decision-answer]'), 'Answer') as HTMLButtonElement).disabled).toBe(false);
  });
});

// ─── Now · Lanes ────────────────────────────────────────────────────────────

describe('Relay · Now — the stat filters choose what Now shows (CONTRACT §1c-0, #422)', () => {
  const stat = (c: HTMLElement, id: string) => need(one(c, `[data-screen-stats] [data-stat="${id}"]`), `the ${id} stat`);
  const busy = () =>
    F.now({
      decisions: [F.decision()],
      lanes: [
        F.lane(),
        F.lane({ id: 'l2', code: 'L2', title: 'Billing', lease_state: 'expired', task: { ...F.lane().task!, id: 't2' } }),
        F.lane({ id: 'l4', code: 'L4', title: 'ER diagrams', task: { ...F.lane().task!, id: 't4', state: 'approved', state_changed_at: F.minutesAgo(52) } }),
      ],
    });
  const rows = (c: HTMLElement) => all(c, '[data-lane-row]').map((r) => r.getAttribute('data-lane-row'));

  it('Waiting replaces the lanes with every wait, oldest first; Needs you and Completed hide; Clear restores', async () => {
    const { c } = await open({ now: F.ok(busy()) });
    stat(c, 'waiting').click();
    await tick();
    const waiting = need(section(c, 'waiting'), 'the waiting section');
    expect(text(one(waiting, '[data-section-heading]'))).toBe('Waiting · oldest first · 1');
    expect(all(waiting, '[data-wait-row]').map((r) => r.getAttribute('data-wait-row'))).toEqual(['t4']);
    expect(section(c, 'lanes')).toBeNull();
    expect(section(c, 'needs-you')).toBeNull();
    expect(section(c, 'completed')).toBeNull();
    expect(text(one(waiting, '[data-showing]'))).toContain('Waiting');
    need(one(waiting, '[data-showing] button'), 'Clear').click();
    await tick();
    expect(section(c, 'waiting')).toBeNull();
    expect(section(c, 'lanes')).not.toBeNull();
    expect(section(c, 'needs-you')).not.toBeNull();
  });

  it('Working and Blocked narrow the lanes; Completed today shows the completed alone', async () => {
    const { c } = await open({ now: F.ok(busy()) });
    stat(c, 'working').click();
    await tick();
    expect(rows(c)).toEqual(['L1']);
    expect(section(c, 'completed')).toBeNull();
    stat(c, 'blocked').click();
    await tick();
    expect(rows(c)).toEqual(['L2']);
    stat(c, 'done').click();
    await tick();
    expect(section(c, 'lanes')).toBeNull();
    expect(section(c, 'completed')).not.toBeNull();
  });
});

describe('Relay · Now — Lanes (#422: LaneList from the selectors)', () => {
  const crew = F.fiveLaneCrew;
  const state = (c: HTMLElement, code: string) => text(one(c, `[data-lane-row="${code}"] [data-part="state"]`));

  it('one card, one row per lane: objective over codename · assistant, the one task, its state, the time on it', async () => {
    const { c } = await open({ now: F.ok(crew()) });
    const lanes = need(section(c, 'lanes'), 'the lanes section');
    expect(text(one(lanes, '[data-section-heading]'))).toBe('Lanes · 5');
    const card = need(one(lanes, 'ul[data-lanes-card]'), 'the lanes card');
    expect(all(card, ':scope > li[data-lane-row]').map((r) => r.getAttribute('data-lane-row'))).toEqual(['L0', 'L1', 'L3', 'L2', 'L5']);

    const l1 = need(one(lanes, '[data-lane-row="L1"]'), '[data-lane-row="L1"]');
    // No codename yet (#395): the code stands in, beside the assistant.
    expect(text(one(l1, '[data-part="name"]'))).toBe('Diagrams L1 · Claude Code 2.1');
    expect(text(one(l1, '[data-part="task"]'))).toBe('Structure view on the shell (tests r1)');
    expect(state(c, 'L1')).toBe('working');
    expect(text(one(l1, '[data-part="when"]'))).toBe('38 min');

    expect(state(c, 'L3')).toBe('needs your answer');
    expect(state(c, 'L2')).toBe('lease lost');
    const idle = need(one(lanes, '[data-lane-row="L5"]'), '[data-lane-row="L5"]');
    expect(text(one(idle, '[data-part="task"]'))).toBe('Nothing assigned');
    expect(state(c, 'L5')).toBe('idle');
  });

  it('the role mark: 指 for the coordinator, 手 for a contributor', async () => {
    const { c } = await open({ now: F.ok(crew()) });
    expect(text(one(c, '[data-lane-row="L0"] [data-lane-role]'))).toBe('指');
    expect(text(one(c, '[data-lane-row="L1"] [data-lane-role]'))).toBe('手');
  });

  it('an assistant that reported no version reads its family alone, never blank', async () => {
    const { c } = await open({
      now: F.ok(F.now({ lanes: [F.lane({ code: 'L3', family: 'cursor', version: null, model: null })] })),
    });
    const row = need(one(c, '[data-lane-row="L3"]'), '[data-lane-row="L3"]');
    expect(text(one(row, '[data-part="name"]'))).toBe('Diagrams L3 · Cursor');
    expect(text(row)).not.toMatch(/null|undefined/);
  });

  it('a lane on a task is one link to the task view; an idle lane opens nothing', async () => {
    const { c } = await open({ now: F.ok(crew()) });
    const l1 = all(c, '[data-lane-row="L1"] a');
    expect(l1).toHaveLength(1);
    expect(l1[0].getAttribute('href')).toBe(`/workspace/w1/relay/${F.CREW_ID}/t-structure`);
    expect(one(c, '[data-lane-row="L5"] a')).toBeNull();
  });
});

// ─── Now · Completed today ──────────────────────────────────────────────────

describe('Relay · Now — Completed today (#422: CompletedList from the selectors)', () => {
  it('one row each: time, title, objective · codename, merged or approved — one link to the task', async () => {
    const { c } = await open();
    const done = need(section(c, 'completed'), 'the completed section');
    expect(text(one(done, '[data-section-heading]'))).toBe('Completed today · 2');
    const rows = all(done, '[data-completed-row]');
    expect(rows).toHaveLength(2);
    expect(text(one(rows[0], '[data-part="title"]'))).toBe('Diagrams header on ScreenShell');
    expect(text(one(rows[0], '[data-part="lane"]'))).toBe('Diagrams · L1');
    expect(text(one(rows[0], '[data-part="outcome"]'))).toBe('merged');
    expect(text(one(rows[1], '[data-part="outcome"]'))).toBe('approved');
    const links = all(rows[0], 'a');
    expect(links).toHaveLength(1);
    expect(links[0].getAttribute('href')).toBe(`/workspace/w1/relay/${F.CREW_ID}/t-header`);
  });
});

// ─── Queue ──────────────────────────────────────────────────────────────────

describe('Relay · Queue', () => {
  it('the Queue button carries its count (the badge and the accessible name), and opens Triage, then Backlog', async () => {
    const { c } = await open();
    const queueTab = tab(c, 'Queue');
    expect(queueTab).not.toBeNull();
    // The badge is drawn from `data-count`; the name folds the count in ("Queue · 4").
    expect(queueTab!.getAttribute('data-count')).toBe('4');
    expect(queueTab!.getAttribute('aria-label')).toBe('Queue · 4');
    expect(isActive(tab(c, 'Now'))).toBe(true);

    queueTab!.click();
    await tick();
    const triage = section(c, 'triage');
    const backlog = section(c, 'backlog');
    expect(triage).not.toBeNull();
    expect(backlog).not.toBeNull();
    expect(triage!.compareDocumentPosition(backlog!) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(section(c, 'lanes')).toBeNull();
    expect(isActive(tab(c, 'Queue'))).toBe(true);
    expect(isActive(tab(c, 'Now'))).toBe(false);
  });

  it('a triage row: title, kind first then tags, the ticket button, and +', async () => {
    const { c } = await open();
    need(tab(c, 'Queue'), 'the Queue tab').click();
    await tick();
    const [ticketed, manual] = triageRows(c);

    expect(text(ticketed)).toContain('Sync shows "synced" for a refused repository');
    // The kind first, as a toned pill, then the tags (#423).
    const tags = all(ticketed, '[data-queue-pills] > *').map(text);
    expect(tags[0]).toBe('bug');
    expect(one(ticketed, '[data-queue-pills] [data-relay-pill]')?.getAttribute('data-tone')).toBe('danger');
    expect(tags).toContain('sprint 47');
    const ticket = need(one(ticketed, 'a[data-ticket]'), 'a[data-ticket]');
    expect(ticket.getAttribute('href')).toBe('https://acme.atlassian.net/browse/SEN-58');
    expect(text(ticket)).toContain('SEN-58');
    expect(one(ticketed, '[data-add-to-backlog]')?.getAttribute('aria-label')).toBe(
      'Add to backlog: Sync shows "synced" for a refused repository',
    );

    expect(all(manual, '[data-queue-pills] > *').map(text)).toContain('added by you');
    expect(one(manual, 'a[data-ticket]')).toBeNull();
    expect(one(manual, '[data-add-to-backlog]')).not.toBeNull();
  });

  it('+ adds a tracker task by its identity, then the re-read queue shows it in Backlog', async () => {
    const { c, api } = await open();
    need(tab(c, 'Queue'), 'the Queue tab').click();
    await tick();
    api.tryGetQueue.mockResolvedValue(
      F.ok(
        F.queue({
          triage: [F.queue().triage[1]],
          backlog: [...F.queue().backlog, F.backlogItem({ item_id: 'item-58', title: 'Sync shows "synced" for a refused repository', position: 3 })],
        }),
      ),
    );

    const [ticketed] = triageRows(c);
    need(one(ticketed, '[data-add-to-backlog]'), '[data-add-to-backlog]').click();

    await vi.waitFor(() =>
      expect(text(section(c, 'backlog'))).toContain('Sync shows "synced" for a refused repository'),
    );
    expect(api.tryAddToBacklog).toHaveBeenCalledTimes(1);
    expect(api.tryAddToBacklog).toHaveBeenCalledWith({ ticket_provider: 'jira', ticket_key: 'SEN-58' });
    expect(api.tryGetQueue).toHaveBeenCalledTimes(2);
    expect(all(need(section(c, 'triage'), 'the triage section'), '[data-queue-row]')).toHaveLength(1);
  });

  it('+ adds a manual item by its id', async () => {
    const { c, api } = await open();
    need(tab(c, 'Queue'), 'the Queue tab').click();
    await tick();
    const [, manual] = triageRows(c);
    need(one(manual, '[data-add-to-backlog]'), '[data-add-to-backlog]').click();
    await vi.waitFor(() => expect(api.tryAddToBacklog).toHaveBeenCalledTimes(1));
    expect(api.tryAddToBacklog).toHaveBeenCalledWith({ item_id: 'item-manual' });
  });

  it('the person adds an open item to triage: the form under the card sends it, and the re-read queue shows it (#423)', async () => {
    const { c, api } = await open();
    api.tryAddTriage.mockResolvedValue(F.ok({ item_id: 'item-9' }));
    need(tab(c, 'Queue'), 'the Queue tab').click();
    await tick();
    api.tryGetQueue.mockResolvedValue(F.ok(F.queue({ triage: [...F.queue().triage, F.manualItem({ item_id: 'item-9', title: 'Hook skips the dōjō tests' })] })));
    const form = need(one(section(c, 'triage')!, '[data-add-item]'), 'the add-an-item form');
    const title = need(one(form, 'input[data-add-item-title]'), 'the title') as HTMLInputElement;
    title.value = 'Hook skips the dōjō tests';
    title.dispatchEvent(new Event('input', { bubbles: true }));
    await tick();
    need(one(form, '[data-add-item-submit]'), 'Add').click();
    await vi.waitFor(() => expect(text(section(c, 'triage'))).toContain('Hook skips the dōjō tests'));
    expect(api.tryAddTriage).toHaveBeenCalledWith({ title: 'Hook skips the dōjō tests', kind: 'issue', tags: [], repo_key: null, workspace_id: 'w1' });
  });

  it('a refused + says why and keeps the row in Triage', async () => {
    const { c, api } = await open();
    api.tryAddToBacklog.mockResolvedValue(F.fail(500, 'Internal Server Error'));
    need(tab(c, 'Queue'), 'the Queue tab').click();
    await tick();
    const [ticketed] = triageRows(c);
    need(one(ticketed, '[data-add-to-backlog]'), '[data-add-to-backlog]').click();

    await vi.waitFor(() => expect(text(one(c, '[data-add-error]'))).toContain('Internal Server Error'));
    expect(all(need(section(c, 'triage'), 'the triage section'), '[data-queue-row]')).toHaveLength(2);
  });

  it('Backlog is in backlog order (position), with no + and no rank numbers', async () => {
    const { c } = await open({
      queue: F.ok(
        F.queue({
          // Out of order on the wire, and in reverse alphabetical order by position:
          // a screen that keeps the wire order, or sorts by title, fails.
          backlog: [
            F.backlogItem({ item_id: 'b2', title: 'Billing export', position: 2 }),
            F.backlogItem({ item_id: 'b1', title: 'Workspace hub', position: 1 }),
          ],
        }),
      ),
    });
    need(tab(c, 'Queue'), 'the Queue tab').click();
    await tick();
    const backlog = need(section(c, 'backlog'), 'the backlog section');
    const titles = all(backlog, '[data-queue-row]').map((r) => text(one(r, '[data-queue-title]')));
    expect(titles).toEqual(['Workspace hub', 'Billing export']);
    expect(one(backlog, '[data-add-to-backlog]')).toBeNull();
    expect(text(backlog)).not.toMatch(/waiting|#1\b|\b1\./);
  });

  it('a failed queue read is its error on Queue, never an empty queue or a 0 count', async () => {
    const { c } = await open({ queue: F.fail(503, 'Service Unavailable') });
    // Now still draws: only the queue's half failed.
    expect(section(c, 'lanes')).not.toBeNull();
    const queueTab = need(tab(c, 'Queue'), 'the Queue tab');
    expect(text(queueTab)).not.toMatch(/\d/);

    queueTab.click();
    await tick();
    expect(text(one(c, '[data-relay-queue-error]'))).toContain('Service Unavailable');
    expect(section(c, 'triage')).toBeNull();
    expect(section(c, 'backlog')).toBeNull();
    expect(text(c)).not.toContain('Triage · 0');
  });

  it('the queue’s Retry re-reads the queue only, and recovers', async () => {
    const { c, api } = await open({ queue: F.fail(503, 'Service Unavailable') });
    api.tryGetQueue.mockResolvedValue(F.ok(F.queue()));
    api.tryGetNow.mockClear();
    need(tab(c, 'Queue'), 'the Queue tab').click();
    await tick();

    need(one(c, '[data-relay-queue-error] [data-action="retry"]'), '[data-relay-queue-error] [data-action="retry"]').click();
    await vi.waitFor(() => expect(section(c, 'triage')).not.toBeNull());
    expect(one(c, '[data-relay-queue-error]')).toBeNull();
    expect(api.tryGetQueue).toHaveBeenLastCalledWith({ workspace_id: 'w1' });
    expect(api.tryGetNow).not.toHaveBeenCalled();
  });
});

// ─── Message the crew ───────────────────────────────────────────────────────

describe('Relay — Message the crew', () => {
  async function compose(opts: Parameters<typeof open>[0] = {}) {
    const o = await open(opts);
    need(toolbarButton(o.c, 'Message the crew'), 'the Message the crew action').click();
    await tick();
    return o;
  }
  const field = (c: HTMLElement, name: 'to' | 'about' | 'text' | 'send') => one(c, `[data-compose-${name}]`);

  it('a labelled toolbar action opens the compose band', async () => {
    const { c } = await open();
    expect(one(c, '[data-compose]')).toBeNull();
    const button = toolbarButton(c, 'Message the crew');
    expect(button).not.toBeNull();
    button!.click();
    await tick();
    const band = one(c, '[data-compose]');
    expect(band).not.toBeNull();
    expect(field(c, 'to')).not.toBeNull();
    expect(field(c, 'about')).not.toBeNull();
    expect(field(c, 'text')).not.toBeNull();
    expect(text(field(c, 'send'))).toBe('Queue message');
  });

  it('To: the coordinator, each contributor lane, everyone; About: no particular task, or a crew task', async () => {
    const { c } = await compose();
    const to = field(c, 'to') as HTMLSelectElement;
    expect([...to.options].map((o) => o.value)).toEqual(['coordinator', F.LANE.L1, 'everyone']);
    expect([...to.options].map((o) => o.textContent?.trim())[1]).toContain('L1 · Diagrams');
    const about = field(c, 'about') as HTMLSelectElement;
    expect([...about.options].map((o) => o.value)).toEqual(['', 't-review-er', 't-structure']);
    expect(about.options[0].textContent?.trim()).toBe('No particular task');
  });

  it('Queue message sends To, About and the text for this crew, then says it is queued', async () => {
    const { c, api } = await compose();
    setValue(field(c, 'to'), F.LANE.L1, 'change');
    setValue(field(c, 'about'), 't-structure', 'change');
    setValue(field(c, 'text'), 'Keep the old keyboard shortcut working.');
    await tick();
    need(field(c, 'send'), 'Queue message').click();

    await vi.waitFor(() => expect(one(c, '[data-compose-queued]')).not.toBeNull());
    expect(api.trySendMessage).toHaveBeenCalledTimes(1);
    expect(api.trySendMessage).toHaveBeenCalledWith({
      crew_id: F.CREW_ID,
      to: { kind: 'lane', lane_id: F.LANE.L1 },
      task_id: 't-structure',
      text: 'Keep the old keyboard shortcut working.',
      surface: 'desktop',
    });
    expect((field(c, 'text') as HTMLTextAreaElement).value).toBe('');
  });

  it('to the coordinator or everyone, about no particular task', async () => {
    const { c, api } = await compose();
    setValue(field(c, 'to'), 'coordinator', 'change');
    setValue(field(c, 'about'), '', 'change');
    setValue(field(c, 'text'), 'Pause after this round.');
    await tick();
    need(field(c, 'send'), 'Queue message').click();
    await vi.waitFor(() => expect(api.trySendMessage).toHaveBeenCalledTimes(1));
    expect(api.trySendMessage).toHaveBeenLastCalledWith(
      expect.objectContaining({ to: { kind: 'coordinator' }, task_id: null }),
    );

    await vi.waitFor(() => expect(one(c, '[data-compose-queued]')).not.toBeNull());
    setValue(field(c, 'to'), 'everyone', 'change');
    setValue(field(c, 'text'), 'Stop for the day.');
    await tick();
    need(field(c, 'send'), 'Queue message').click();
    await vi.waitFor(() => expect(api.trySendMessage).toHaveBeenCalledTimes(2));
    expect(api.trySendMessage).toHaveBeenLastCalledWith(expect.objectContaining({ to: { kind: 'everyone' } }));
  });

  it('an empty message, or one over 600 characters, cannot be queued', async () => {
    const { c } = await compose();
    const send = () => need(field(c, 'send'), 'Queue message') as HTMLButtonElement;
    expect(send().disabled).toBe(true);
    setValue(field(c, 'text'), '   ');
    await tick();
    expect(send().disabled).toBe(true);
    setValue(field(c, 'text'), 'x'.repeat(601));
    await tick();
    expect(send().disabled).toBe(true);
    setValue(field(c, 'text'), 'x'.repeat(600));
    await tick();
    expect(send().disabled).toBe(false);
  });

  it('a refused message says why and keeps the text', async () => {
    const { c, api } = await compose();
    api.trySendMessage.mockResolvedValue(F.fail(503, 'daemon busy'));
    setValue(field(c, 'text'), 'Keep the shortcut.');
    await tick();
    need(field(c, 'send'), 'Queue message').click();

    await vi.waitFor(() => expect(text(one(c, '[data-compose-error]'))).toContain('daemon busy'));
    expect(one(c, '[data-compose-queued]')).toBeNull();
    expect((field(c, 'text') as HTMLTextAreaElement).value).toBe('Keep the shortcut.');
  });

  it('two presses of Queue message send one message', async () => {
    const { c, api } = await compose();
    const pending = F.deferred<ApiResult<{ message_id: string }>>();
    api.trySendMessage.mockReturnValue(pending.promise);
    setValue(field(c, 'text'), 'Once only.');
    await tick();
    const send = field(c, 'send') as HTMLButtonElement;
    send.click();
    send.click();
    pending.resolve(F.ok({ message_id: 'msg-1' }));
    await vi.waitFor(() => expect(one(c, '[data-compose-queued]')).not.toBeNull());
    expect(api.trySendMessage).toHaveBeenCalledTimes(1);
  });
});

// ─── States ─────────────────────────────────────────────────────────────────

describe('Relay — a failed read', () => {
  const down = () => F.fail(0, 'connection refused');

  it('is the fault Notice with its code, never "No crew yet" and never empty lanes', async () => {
    const { c } = await open({ now: down() });
    const notice = one(c, '[data-relay-error]');
    expect(notice).not.toBeNull();
    expect(notice!.getAttribute('data-tone')).toBe('fault');
    expect(text(notice)).toContain("Couldn't read the crew");
    expect(text(notice)).toContain('connection refused');
    expect(text(one(notice!, '[data-part="code"]'))).toBe('daemon_unreachable');

    expect(text(c)).not.toContain('No crew yet');
    expect(one(c, '[data-paste-line]')).toBeNull();
    expect(section(c, 'lanes')).toBeNull();
    expect(text(c)).not.toContain('Lanes · 0');
    expect(toolbarButton(c, 'Message the crew')).toBeNull();
  });

  it('Retry reads once for this workspace, and the lanes replace the notice', async () => {
    const { c, api } = await open({ now: down(), workspace: 'w2' });
    api.tryGetNow.mockClear();
    api.tryGetNow.mockResolvedValue(F.ok(F.now()));

    need(one(c, '[data-relay-error] [data-action="retry"]'), '[data-relay-error] [data-action="retry"]').click();
    await vi.waitFor(() => expect(section(c, 'lanes')).not.toBeNull());
    expect(one(c, '[data-relay-error]')).toBeNull();
    expect(api.tryGetNow).toHaveBeenCalledTimes(1);
    expect(api.tryGetNow).toHaveBeenCalledWith({ workspace_id: 'w2' });
  });

  it('a Retry that fails again shows the new reason, not the old one, and not an empty crew', async () => {
    const { c, api } = await open({ now: down() });
    api.tryGetNow.mockResolvedValue(F.fail(502, 'Bad Gateway'));

    need(one(c, '[data-relay-error] [data-action="retry"]'), '[data-relay-error] [data-action="retry"]').click();
    await vi.waitFor(() => expect(text(one(c, '[data-relay-error]'))).toContain('Bad Gateway'));
    expect(text(one(c, '[data-relay-error]'))).not.toContain('connection refused');
    expect(section(c, 'lanes')).toBeNull();
    expect(text(c)).not.toContain('No crew yet');
  });

  it('two presses of Retry are one read', async () => {
    const { c, api } = await open({ now: down() });
    const pending = F.deferred<ApiResult<RelayNow>>();
    api.tryGetNow.mockClear();
    api.tryGetNow.mockReturnValue(pending.promise);

    const retry = need(one(c, '[data-relay-error] [data-action="retry"]'), 'the Retry on the error notice');
    retry.click();
    retry.click();
    pending.resolve(F.ok(F.now()));
    await vi.waitFor(() => expect(section(c, 'lanes')).not.toBeNull());
    expect(api.tryGetNow).toHaveBeenCalledTimes(1);
  });

  it('while the Retry reads, the body shows the loading skeleton', async () => {
    const { c, api } = await open({ now: down() });
    const pending = F.deferred<ApiResult<RelayNow>>();
    api.tryGetNow.mockReturnValue(pending.promise);

    need(one(c, '[data-relay-error] [data-action="retry"]'), '[data-relay-error] [data-action="retry"]').click();
    await tick();
    expect(one(c, '[data-screen-body] [data-component="loading"]')).not.toBeNull();

    pending.resolve(F.ok(F.now()));
    await vi.waitFor(() => expect(section(c, 'lanes')).not.toBeNull());
    expect(one(c, '[data-component="loading"]')).toBeNull();
  });
});

describe('Relay — a navigation replaces what is shown', () => {
  it('the loader’s new answer replaces the old crew', async () => {
    const api = F.fakeRelayApi();
    api.tryGetNow.mockResolvedValue(F.ok(F.now({ lanes: [F.lane({ code: 'L1', title: 'Diagrams' })] })));
    const first = await loadFor('w1', api);
    const { default: Page } = await import(/* @vite-ignore */ PAGE);
    const props = $state({ data: first });
    const m = mountComponent(Page, props as never);
    cleanup.push(m.destroy);
    await vi.waitFor(() => expect(one(m.container, '[data-lane-row="L1"]')).not.toBeNull());

    api.tryGetNow.mockResolvedValue(
      F.ok(F.now({ crews: [F.crew({ id: F.OTHER_CREW_ID, workspace_id: 'w2' })], lanes: [F.lane({ id: 'lane-9', code: 'L9', title: 'Billing' })] })),
    );
    props.data = await loadFor('w2', api);
    await tick();
    await vi.waitFor(() => expect(one(m.container, '[data-lane-row="L9"]')).not.toBeNull());
    expect(one(m.container, '[data-lane-row="L1"]')).toBeNull();
  });
});

describe('Relay — the Crew view with a crew (#424)', () => {
  it('the run card over the members by role, then Add a member for contributors', async () => {
    const { c } = await open({ now: F.ok(F.now({ runs: [F.sprint()], lanes: [F.coordinatorLane(), F.lane()] })) });
    need(tab(c, 'Crew'), 'the Crew button').click();
    await tick();
    const crew = need(section(c, 'crew'), 'the crew section');
    const card = need(one(crew, '[data-run-card]'), 'the run card');
    expect(text(one(card, '[data-run-title]'))).toContain(`Run ${F.RUN_ID.slice(-4)}`);
    expect(all(card, '[data-member]').map((m) => m.getAttribute('data-member'))).toEqual(['L0', 'L1']);
    const add = need(one(crew, '[data-add-member]'), 'Add a member');
    expect(text(one(add, '[data-add-member-title]'))).toBe('Add a member');
    expect(text(one(add, '[data-paste-text]'))).toBe('register with sensei as contributor');
  });

  it('a member’s Message opens the compose band', async () => {
    const { c } = await open({ now: F.ok(F.now({ runs: [F.sprint()] })) });
    need(tab(c, 'Crew'), 'the Crew button').click();
    await tick();
    need(one(c, '[data-member="L1"] [data-member-message]'), 'Message on L1').click();
    await tick();
    expect(one(c, '[data-compose]')).not.toBeNull();
  });
});

describe('Relay — no crew yet', () => {
  it('the info band frames it ("No crew yet", what to do); the Crew view is the landing: set up the crew, step 1 of 2 (#424)', async () => {
    const { c } = await open({ now: F.ok(F.noCrew()) });
    const info = need(one(c, '[data-screen-info]'), '[data-screen-info]');
    expect(text(info)).toContain('No crew yet');
    expect(text(info)).toContain('Set it up below');
    // CONTRACT §1b/§1h (r1): the setup is the Crew view, the landing while there is no
    // run; the Crew button is the pressed one.
    expect(isActive(tab(c, 'Crew'))).toBe(true);
    const crew = need(section(c, 'crew'), 'the crew section');
    const add = need(one(crew, '[data-add-member]'), 'the set-up card');
    expect(text(one(add, '[data-add-member-title]'))).toBe('Start a run: set up your crew');
    expect(text(one(add, '[data-command-step]'))).toBe('Step 1 of 2.');
    expect(all(add, '[data-paste-line]').map((l) => text(one(l, '[data-paste-text]')))).toEqual(['register with sensei as coordinator']);
    expect(text(one(add, '[data-listening]'))).toBe('Listening · 0 of 2 to start a run');
    expect(one(crew, '[data-run-card]')).toBeNull();

    expect(section(c, 'needs-you')).toBeNull();
    expect(section(c, 'lanes')).toBeNull();
    expect(section(c, 'completed')).toBeNull();
    expect(one(c, '[data-relay-error]')).toBeNull();
    expect(toolbarButton(c, 'Message the crew')).toBeNull();
  });

  it('choosing Contributor shows its line; Copy puts the shown line on the clipboard', async () => {
    const writeText = vi.fn(async () => {});
    Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true });
    const { c } = await open({ now: F.ok(F.noCrew()) });
    need(one(c, '[data-role-option="contributor"]'), 'Contributor').click();
    await tick();
    expect(text(one(c, '[data-paste-line] [data-paste-text]'))).toBe('register with sensei as contributor');
    need(one(c, '[data-paste-copy]'), '[data-paste-copy]').click();
    await vi.waitFor(() => expect(writeText).toHaveBeenCalledWith('register with sensei as contributor'));
  });

  it('with no crew the info band cannot be closed', async () => {
    const { c } = await open({ now: F.ok(F.noCrew()) });
    expect(one(c, '[data-screen-info]')).not.toBeNull();
    expect(toolbarButton(c, 'What it’s for')).toBeNull();
  });
});

// ─── The shell (D87) ────────────────────────────────────────────────────────

describe('Relay — the shell (D87)', () => {
  it('rokkit’s shell for workspace.relay: mark 継, title Relay, the four stat cards on Now (§1c-0)', async () => {
    const { c } = await open();
    expect(vi.mocked(ScreenShell)).toHaveBeenCalled();
    const props = shellProps();
    expect(props).toMatchObject({ viewId: 'workspace.relay', title: 'Relay', mark: { kanji: '継' } });
    expect((props.stats as Array<{ id: string }>).map((s) => s.id)).toEqual(['working', 'waiting', 'blocked', 'done']);
    expect(one(c, '[data-screen-shell]')?.getAttribute('data-view')).toBe('workspace.relay');
    expect(text(one(c, '[data-screen-title]'))).toBe('Relay');
    expect(one(c, '[data-screen-stats]')).not.toBeNull();
  });

  it('the context names this crew by the end of its id (a uuid v7 starts with the clock)', async () => {
    const { c } = await open({
      workspace: 'w2',
      now: F.ok(F.now({ crews: [F.crew({ id: F.OTHER_CREW_ID, workspace_id: 'w2' })] })),
    });
    const context = text(one(c, '[data-screen-context]'));
    expect(context).toContain(`crew ${F.OTHER_CREW_ID.slice(-4)}`);
    expect(context).not.toContain(F.CREW_ID.slice(-4));
  });

  it('the info band with a crew: what Relay is, three steps, and Today with first-pass approval', async () => {
    const { c } = await open();
    const info = need(one(c, '[data-screen-info]'), '[data-screen-info]');
    expect(info).not.toBeNull();
    expect(text(info)).toContain('Your crew of assistant sessions');
    expect(all(info, '[data-screen-info-step]')).toHaveLength(3);
    expect(text(info)).toContain('Today');
    expect(text(info)).toContain('2 completed');
    expect(text(info)).toContain('1 of 5 passed review first time');
    expect(one(info, '[data-paste-line]')).toBeNull();
  });

  it('first-pass approval nobody reported reads "not reported", never "0 of"', async () => {
    const { c } = await open({
      now: F.ok(F.now({ today: { completed: 2, reviewed: null, first_pass_approved: null } })),
    });
    const info = text(one(c, '[data-screen-info]'));
    expect(info).toContain('not reported');
    expect(info).not.toMatch(/0 of/);
  });

  it('the marks follow the glyph mode: kanji, or the vocabulary’s icon', async () => {
    glyphs.mode = 'kanji';
    const kanji = await open();
    expect(text(one(kanji.c, '[data-screen-mark]'))).toBe('継');
    expect(text(one(kanji.c, '[data-lane-row="L0"] [data-lane-role]'))).toBe('指');

    glyphs.mode = 'icon';
    const icon = await open();
    expect(one(icon.c, '[data-screen-mark]')?.className).toContain(KANJI_ICONS['継']);
    expect(one(icon.c, '[data-lane-row="L0"] [data-lane-role]')?.className).toMatch(/\bi-/);
    expect(text(one(icon.c, '[data-lane-row="L0"] [data-lane-role]'))).not.toBe('指');
  });

  it('the e2e anchor: data-screen="relay" is the shell itself (no wrapper: the shell fills its container)', async () => {
    const { c } = await open();
    const anchor = one(c, '[data-screen="relay"]');
    expect(anchor).not.toBeNull();
    expect(anchor!.hasAttribute('data-screen-shell'), 'the anchor is the shell root').toBe(true);
    expect(anchor!.classList.contains('h-full'), 'the shell fills its container (#420)').toBe(true);
  });
});

// ─── Footer: resources (lanes.md §8) ────────────────────────────────────────

describe('Relay — the footer resource line', () => {
  it('the build lease holder and its time, disk free, and swap free against its floor', async () => {
    const { c } = await open();
    const footer = text(one(c, '[data-screen-footer]'));
    expect(footer).toContain('Rust build');
    expect(footer).toContain('L1');
    expect(footer).toContain('4 min');
    expect(footer).toContain('Disk 46 GB free');
    expect(footer).toContain('Swap 1.2 GB free (floor 1 GB)');
  });

  it('unreported disk or swap reads "not reported"; no lease reads as no build', async () => {
    const { c } = await open({
      now: F.ok(
        F.now({
          resources: { build_lease: null, disk_free_gb: null, disk_floor_gb: 25, swap_free_gb: null, swap_floor_gb: 1 },
        }),
      ),
    });
    const footer = text(one(c, '[data-screen-footer]'));
    expect(footer).toContain('Disk not reported');
    expect(footer).toContain('Swap not reported');
    expect(footer).toMatch(/no build/i);
    expect(footer).not.toMatch(/\b0 GB\b/);
  });
});

// ─── Phone width (CONTRACT §1f), where jsdom can tell ───────────────────────

describe('Relay — rows at phone width', () => {
  it('a lane row is the mockup’s parts in order: mark, name, task, state, time, chevron (#422)', async () => {
    const { c } = await open();
    const row = need(one(c, '[data-lane-row="L1"]'), '[data-lane-row="L1"]');
    const link = need(one(row, 'a'), 'the row’s link');
    expect([...link.children].map((el) => el.getAttribute('data-part'))).toEqual(['mark', 'name', 'task', 'state', 'when', 'chevron']);
    expect(one(row, '[data-part="mark"] [data-lane-role]')).not.toBeNull();
    expect(text(one(row, '[data-part="name"]'))).toContain('Diagrams');
    expect(text(one(row, '[data-part="task"]'))).toContain('Structure view on the shell');
    expect(text(one(row, '[data-part="when"]'))).toBe('38 min');
  });

  it('a queue row: the title over its pills, the ticket pill and + beside it; on a phone the ticket drops its provider (#423)', async () => {
    const { c } = await open();
    need(tab(c, 'Queue'), 'the Queue tab').click();
    await tick();
    const [row] = triageRows(c);
    expect(one(row, '[data-queue-title]')).not.toBeNull();
    expect(one(row, '[data-queue-pills]')).not.toBeNull();
    expect(one(row, 'a[data-ticket]')).not.toBeNull();
    expect(one(row, '[data-add-to-backlog]')).not.toBeNull();
    const provider = need(one(row, '[data-ticket-provider]'), '[data-ticket-provider]');
    expect(text(provider)).toBe('Jira');
    expect(provider.className).toMatch(/(^|\s)hidden md:inline(\s|$)/);
    expect(text(one(row, '[data-ticket-key]'))).toBe('SEN-58');
  });
});

// ─── r2 (review relay-screen-tests-r1) ──────────────────────────────────────

/** A local "HH:MM" for an instant, as the reader's clock shows it. */
const hhmm = (iso: string) => {
  const d = new Date(iso);
  return `${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`;
};

describe('Relay r2 — the header (F1, F2, L4)', () => {
  it('search is off on Relay (L0 ruling): no search field, no filters band', async () => {
    const { c } = await open();
    expect(vi.mocked(ScreenShell)).toHaveBeenCalled();
    expect(shellProps().search).toBe(false);
    expect(one(c, '[data-screen-search]')).toBeNull();
    expect(one(c, '[data-screen-filters]')).toBeNull();
  });

  it('the header’s controls are exactly Now, Queue, Message the crew and the info toggle', async () => {
    const { c } = await open();
    const header = need(one(c, '[data-screen-header]'), 'the shell header');
    // The stats band lives in the header too; its cards are its own controls (§1c-0).
    const controls = all(header, 'button, a, [role="tab"]')
      .filter((el) => !el.closest('[data-screen-stats]'))
      .map((el) => el.getAttribute('aria-label') || text(el))
      .map((label) => (label.startsWith('Queue') || label.startsWith('Crew') ? label.split(' · ')[0] : label))
      .sort();
    // The three views, Message the crew and the info toggle; no stats / filters / side
    // toggles and no other action (CONTRACT §1a, user r1; the view buttons carry the
    // counts, the info toggle its own label).
    expect(controls).toEqual(['Crew', 'Hide info', 'Message the crew', 'Now', 'Queue']);
  });

  it('the context is the workspace, then the crew', async () => {
    const { c } = await open({
      workspace: 'w2',
      layout: { workspace: { id: 'w2', name: 'Billing service' }, workspaceId: 'w2' },
      now: F.ok(F.now({ crews: [F.crew({ id: F.OTHER_CREW_ID, workspace_id: 'w2' })] })),
    });
    expect(text(one(c, '[data-screen-context]'))).toBe(`Billing service · crew ${F.OTHER_CREW_ID.slice(-4)}`);
  });
});

describe('Relay r2 — one write per press (F4)', () => {
  it('two presses of Answer answer once', async () => {
    const { c, api } = await open({ now: F.ok(F.now({ decisions: [F.decision()] })) });
    const pending = F.deferred<ApiResult<{ action_id: string }>>();
    api.tryAnswerDecision.mockReturnValue(pending.promise);
    api.tryGetNow.mockResolvedValue(F.ok(F.now({ decisions: [] })));
    const row = need(one(c, '[data-decision]'), 'a decision');
    need(one(row, '[data-option="db"]'), 'the db option').click();
    await tick();
    const answer = need(one(row, '[data-decision-answer]'), 'Answer');
    answer.click();
    answer.click();
    pending.resolve(F.ok({ action_id: 'act-answer' }));
    await vi.waitFor(() => expect(api.tryAnswerDecision).toHaveBeenCalledTimes(1));
    // The event the answer causes takes it out of Needs you (#421).
    relay.streams[0].fire({ action: 'update', entity: 'task', data: { op: 'answer', task_id: 't-structure' } });
    await vi.waitFor(() => expect(one(c, '[data-decision]')).toBeNull());
    expect(api.tryAnswerDecision).toHaveBeenCalledTimes(1);
  });

  it('two presses of + add once', async () => {
    const { c, api } = await open();
    const pending = F.deferred<ApiResult<{ item_id: string }>>();
    api.tryAddToBacklog.mockReturnValue(pending.promise);
    need(tab(c, 'Queue'), 'the Queue tab').click();
    await tick();
    const [ticketed] = triageRows(c);
    const add = need(one(ticketed, '[data-add-to-backlog]'), '+ on the tracker row');
    add.click();
    add.click();
    pending.resolve(F.ok({ item_id: 'item-58' }));
    await vi.waitFor(() => expect(api.tryGetQueue).toHaveBeenCalledTimes(2));
    expect(api.tryAddToBacklog).toHaveBeenCalledTimes(1);
  });
});

describe('Relay r2 — Message the crew across five lanes (F6)', () => {
  it('To lists every contributor lane in the view’s order; About skips the idle lane', async () => {
    const { c } = await open({ now: F.ok(F.fiveLaneCrew()) });
    need(toolbarButton(c, 'Message the crew'), 'the Message the crew action').click();
    await tick();
    const to = need(one(c, '[data-compose-to]'), 'To') as HTMLSelectElement;
    expect([...to.options].map((o) => o.value)).toEqual([
      'coordinator',
      F.LANE.L1,
      F.LANE.L3,
      F.LANE.L2,
      F.LANE.L5,
      'everyone',
    ]);
    expect(to.options[2].textContent).toContain('L3 · Activity logs');
    const about = need(one(c, '[data-compose-about]'), 'About') as HTMLSelectElement;
    expect([...about.options].map((o) => o.value)).toEqual(['', 't-review-er', 't-structure', 't-logs', 't-billing']);
  });
});

describe('Relay r2 — what drives a lane’s state pill (F7)', () => {
  const task = (over: Partial<NonNullable<ReturnType<typeof F.lane>['task']>>) => ({
    id: 't-x',
    title: 'A task',
    repo_key: F.REPO,
    workspace_id: 'w1' as string | null,
    phase: 'impl' as const,
    state: 'working' as const,
    round: 1,
    state_changed_at: F.minutesAgo(12),
    current_step: null as string | null,
    steps_done: 0,
    steps_total: 0,
    needs_answer: false,
    ...over,
  });

  it('blocked on a dependency reads "blocked"; only an open decision reads "needs your answer"', async () => {
    const { c } = await open({
      now: F.ok(
        F.now({
          lanes: [
            F.lane({ id: F.LANE.L4, code: 'L4', title: 'Metrics', task: task({ id: 't-dep', state: 'blocked' }) }),
            F.lane({ id: F.LANE.L3, code: 'L3', title: 'Logs', task: task({ id: 't-ask', state: 'blocked', needs_answer: true }) }),
          ],
        }),
      ),
    });
    expect(text(one(c, '[data-lane-row="L4"] [data-part="state"]'))).toBe('blocked');
    expect(text(one(c, '[data-lane-row="L3"] [data-part="state"]'))).toBe('needs your answer');
  });

  it('the lane lease drives "lease lost", with the time it expired; a task interrupted under a live lease reads "interrupted"', async () => {
    const expired = new Date(Date.now() - 20 * 60_000).toISOString();
    const { c } = await open({
      now: F.ok(
        F.now({
          lanes: [
            F.lane({ id: F.LANE.L2, code: 'L2', title: 'Billing', lease_state: 'expired', lease_expires_at: expired, task: task({ id: 't-b', state: 'working' }) }),
            F.lane({ id: F.LANE.L5, code: 'L5', title: 'Docs', lease_state: 'granted', task: task({ id: 't-d', state: 'interrupted' }) }),
          ],
        }),
      ),
    });
    const lost = need(one(c, '[data-lane-row="L2"]'), 'the L2 row');
    expect(text(one(lost, '[data-part="state"]'))).toBe('lease lost');
    expect(text(lost)).toContain(`at ${hhmm(expired)}`);
    expect(text(one(c, '[data-lane-row="L5"] [data-part="state"]'))).toBe('interrupted');
  });

  it('a limit-paused lane reads "paused" with scope and resume time — never "lease lost" on a live lease', async () => {
    const resumes = new Date(Date.now() + 120 * 60_000).toISOString();
    const { c } = await open({
      now: F.ok(
        F.now({
          lanes: [
            F.lane({ id: F.LANE.L2, code: 'L2', title: 'Billing', lease_state: 'granted', limit_scope: 'weekly', limit_resumes_at: resumes, limit_note: null, task: task({ id: 't-b', state: 'working' }) }),
            F.lane({ id: F.LANE.L5, code: 'L5', title: 'Docs', lease_state: 'granted', limit_scope: 'daily', limit_resumes_at: null, limit_note: null, task: null }),
          ],
        }),
      ),
    });
    const paused = need(one(c, '[data-lane-row="L2"]'), 'the L2 row');
    expect(text(one(paused, '[data-part="state"]'))).toBe('paused');
    expect(text(paused)).toContain('weekly');
    expect(text(paused)).toContain(`until ${hhmm(resumes)}`);
    const unknown = need(one(c, '[data-lane-row="L5"]'), 'the L5 row');
    expect(text(one(unknown, '[data-part="state"]'))).toBe('paused');
    expect(text(unknown)).toContain('resume unknown');
  });
});

describe('Relay r2 — Completed today rows (L1)', () => {
  it('each row leads with its time, and "done" (nothing to merge) has its own label', async () => {
    const at = F.minutesAgo(90);
    const { c } = await open({
      now: F.ok(
        F.now({
          completed_today: [
            F.completed({ completed_at: at }),
            F.completed({ task_id: 't-mock', title: 'Relay mockup', outcome: 'done', completed_at: F.minutesAgo(30) }),
          ],
        }),
      ),
    });
    const rows = all(need(section(c, 'completed'), 'the completed section'), '[data-completed-row]');
    expect(rows).toHaveLength(2);
    expect(text(rows[0]).startsWith(hhmm(at))).toBe(true);
    expect(text(one(rows[1], '[data-part="outcome"]'))).toBe('done');
  });
});

describe('Relay follow-up — a re-read that fails after a write (N8)', () => {
  it('an answer whose re-read fails is the fault Notice, never a stale Needs you', async () => {
    const { c, api } = await open({ now: F.ok(F.now({ decisions: [F.decision()] })) });
    api.tryGetNow.mockResolvedValueOnce(F.fail(502, 'the daemon dropped the read'));

    const row = need(one(c, '[data-decision]'), 'the decision');
    need(one(row, '[data-option="file"]'), 'the file option').click();
    await tick();
    need(one(row, '[data-decision-answer]'), 'Answer').click();
    await vi.waitFor(() => expect(api.tryAnswerDecision).toHaveBeenCalledTimes(1));
    // The answer's event names ids only today: Now is read again, and that read fails.
    relay.streams[0].fire({ action: 'update', entity: 'task', data: { op: 'answer', task_id: 't-structure' } });

    await vi.waitFor(() => expect(one(c, '[data-relay-error]')).not.toBeNull());
    const notice = need(one(c, '[data-relay-error]'), 'the fault Notice after a failed re-read');
    expect(text(notice)).toContain('the daemon dropped the read');
    expect(section(c, 'lanes')).toBeNull();

    api.tryGetNow.mockResolvedValue(F.ok(F.now({ decisions: [] })));
    need(one(notice, '[data-action="retry"]'), 'the Notice’s Retry').click();
    await vi.waitFor(() => expect(section(c, 'lanes')).not.toBeNull());
    expect(one(c, '[data-relay-error]')).toBeNull();
  });

  it('an add whose queue re-read fails is the queue’s fault Notice', async () => {
    const { c, api } = await open();
    need(tab(c, 'Queue'), 'the Queue tab').click();
    await tick();
    api.tryGetQueue.mockResolvedValueOnce(F.fail(503, 'the daemon dropped the read'));

    const row = need(
      triageRows(c).find((r) => one(r, '[data-add-to-backlog]')),
      'a triage row with +',
    );
    need(one(row, '[data-add-to-backlog]'), '[data-add-to-backlog]').click();

    await vi.waitFor(() => expect(one(c, '[data-relay-queue-error]')).not.toBeNull());
    const notice = need(one(c, '[data-relay-queue-error]'), 'the queue’s fault Notice after a failed re-read');
    expect(text(notice)).toContain('the daemon dropped the read');
    expect(section(c, 'backlog')).toBeNull();

    api.tryGetQueue.mockResolvedValue(F.ok(F.queue()));
    need(one(notice, '[data-action="retry"]'), 'Retry').click();
    await vi.waitFor(() => expect(section(c, 'triage')).not.toBeNull());
    expect(one(c, '[data-relay-queue-error]')).toBeNull();
  });
});
