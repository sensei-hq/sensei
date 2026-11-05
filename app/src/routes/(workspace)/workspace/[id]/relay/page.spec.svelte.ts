// @vitest-environment jsdom
// Workspace · Relay — the screen that replaces Intake (D90, relay slice 1 item 4).
//
// Contract: coord/mockups/relay-lanes/CONTRACT.md §1 (the screen), §3 (the shapes);
// data: docs/prd/modules/03-delivery/21-relay/lanes-data.md (`relay.now`, the queue).
//
// Phase A: every test is a disabled red (`pending: relay-screen`). The route does not
// exist yet, so the loader and the page are imported inside each test — a static
// import would fail the whole file, and the live specs beside it with it.
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

const relay = vi.hoisted(() => ({ api: null as unknown }));
vi.mock('$lib/relay-api.js', () => ({ relayApi: vi.fn(() => relay.api) }));

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

/** A header tab by its label ("Now", "Queue"), wherever the header draws it. */
function tab(c: HTMLElement, label: string): HTMLElement | null {
  const header = one(c, '[data-screen-header]');
  if (!header) return null;
  return (
    all(header, '[role="tab"], button, a').find((el) => text(el).startsWith(label)) ?? null
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
  it.skip('pending: relay-screen — reads this workspace’s crew and queue, and the page reads nothing of its own', async () => {
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

describe('Relay · Now — Needs you', () => {
  it.skip('pending: relay-screen — is absent when nothing needs the person', async () => {
    // r2 L6: the default crew has triage (as the default queue does), so this one has none.
    const { c } = await open({ now: F.ok(F.now({ triage: [], decisions: [] })), queue: F.ok(F.queue({ triage: [] })) });
    expect(section(c, 'lanes')).not.toBeNull();
    expect(section(c, 'needs-you')).toBeNull();
  });

  it.skip('pending: relay-screen — each decision: question, context, options with its own recommended one marked, Answer, and its task', async () => {
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

  it.skip('pending: relay-screen — "N to triage — from …" counts every source, names them, and opens Queue', async () => {
    const { c } = await open({
      now: F.ok(
        F.now({
          triage: [
            { provider: 'jira', sprint: 'sprint 47', count: 2 },
            { provider: 'github', sprint: 'milestone 11', count: 1 },
            { provider: 'manual', sprint: null, count: 1 },
          ],
        }),
      ),
    });
    const line = one(c, '[data-triage-line]');
    expect(line).not.toBeNull();
    expect(text(line)).toContain('4 to triage');
    expect(text(line)).toContain('Jira sprint 47');
    expect(text(line)).toContain('GitHub milestone 11');

    line!.click();
    await tick();
    expect(section(c, 'triage')).not.toBeNull();
    expect(section(c, 'lanes')).toBeNull();
    expect(isActive(tab(c, 'Queue'))).toBe(true);
  });

  it.skip('pending: relay-screen — Answer sends the option the person picked, and the decision leaves Needs you', async () => {
    const { c, api } = await open({ now: F.ok(F.now({ decisions: [F.decision()] })) });
    api.tryGetNow.mockResolvedValue(F.ok(F.now({ decisions: [] })));

    const row = need(one(c, '[data-decision]'), '[data-decision]');
    need(one(row, '[data-option="file"]'), '[data-option="file"]').click();
    await tick();
    need(one(row, '[data-decision-answer]'), '[data-decision-answer]').click();

    await vi.waitFor(() => expect(one(c, '[data-decision]')).toBeNull());
    expect(api.tryAnswerDecision).toHaveBeenCalledTimes(1);
    expect(api.tryAnswerDecision).toHaveBeenCalledWith({
      decision_id: 'ask-1',
      option: 'file',
      note: null,
      surface: 'desktop',
    });
    // The answer is the daemon's to record; the screen re-reads rather than guessing.
    expect(api.tryGetNow).toHaveBeenCalledTimes(2);
  });

  it.skip('pending: relay-screen — a refused answer says why, keeps the decision, and can be sent again', async () => {
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

describe('Relay · Now — Lanes', () => {
  const crew = F.fiveLaneCrew;

  it.skip('pending: relay-screen — one row per lane: its objective, member, the one task, its state and the time on it', async () => {
    const { c } = await open({ now: F.ok(crew()) });
    const lanes = need(section(c, 'lanes'), 'the lanes section');
    expect(lanes).not.toBeNull();
    expect(text(lanes)).toContain('Lanes · 5');
    expect(all(lanes, '[data-lane-row]').map((r) => r.getAttribute('data-lane-row'))).toEqual([
      'L0',
      'L1',
      'L3',
      'L2',
      'L5',
    ]);

    const l1 = need(one(lanes, '[data-lane-row="L1"]'), '[data-lane-row="L1"]');
    expect(text(l1)).toContain('Diagrams');
    expect(text(l1)).toContain('L1');
    expect(text(l1)).toContain('2.1');
    expect(text(l1)).toContain('Structure view on the shell');
    expect(text(one(l1, '[data-state]'))).toBe('working');
    expect(text(l1)).toContain('38 min');

    expect(text(one(lanes, '[data-lane-row="L3"] [data-state]'))).toBe('needs your answer');
    expect(text(one(lanes, '[data-lane-row="L2"] [data-state]'))).toBe('lease lost');
    const idle = need(one(lanes, '[data-lane-row="L5"]'), '[data-lane-row="L5"]');
    expect(text(idle)).toContain('Nothing assigned');
    expect(text(one(idle, '[data-state]'))).toBe('idle');
  });

  it.skip('pending: relay-screen — the role mark: 指 for the coordinator, 手 for a contributor', async () => {
    const { c } = await open({ now: F.ok(crew()) });
    expect(text(one(c, '[data-lane-row="L0"] [data-lane-role]'))).toBe('指');
    expect(text(one(c, '[data-lane-row="L1"] [data-lane-role]'))).toBe('手');
  });

  it.skip('pending: relay-screen — a model the lane did not report reads "model not reported", never blank', async () => {
    const { c } = await open({
      now: F.ok(F.now({ lanes: [F.lane({ code: 'L3', family: 'muse', version: null, model: null })] })),
    });
    const row = need(one(c, '[data-lane-row="L3"]'), '[data-lane-row="L3"]');
    expect(text(row)).toContain('model not reported');
    expect(text(row)).not.toMatch(/null|undefined/);
  });

  it.skip('pending: relay-screen — a lane on a task opens the task view; an idle lane opens nothing', async () => {
    const { c } = await open({ now: F.ok(crew()) });
    expect(one(c, `[data-lane-row="L1"] a[href="/workspace/w1/relay/${F.CREW_ID}/t-structure"]`)).not.toBeNull();
    expect(one(c, '[data-lane-row="L5"] a')).toBeNull();
  });
});

// ─── Now · Completed today ──────────────────────────────────────────────────

describe('Relay · Now — Completed today', () => {
  it.skip('pending: relay-screen — one row each: title, lane, merged or approved, and it opens the task', async () => {
    const { c } = await open();
    const done = need(section(c, 'completed'), 'the completed section');
    expect(done).not.toBeNull();
    expect(text(done)).toContain('Completed today · 2');
    const rows = all(done, '[data-completed-row]');
    expect(rows).toHaveLength(2);
    expect(text(rows[0])).toContain('Diagrams header on ScreenShell');
    expect(text(rows[0])).toContain('Diagrams');
    expect(text(one(rows[0], '[data-state]'))).toBe('merged');
    expect(text(one(rows[1], '[data-state]'))).toBe('approved');
    expect(one(rows[0], `a[href="/workspace/w1/relay/${F.CREW_ID}/t-header"]`)).not.toBeNull();
  });
});

// ─── Queue ──────────────────────────────────────────────────────────────────

describe('Relay · Queue', () => {
  it.skip('pending: relay-screen — the Queue tab shows its count, and opens Triage, then Backlog', async () => {
    const { c } = await open();
    const queueTab = tab(c, 'Queue');
    expect(queueTab).not.toBeNull();
    expect(text(queueTab)).toContain('4');
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

  it.skip('pending: relay-screen — a triage row: title, kind first then tags, the ticket button, and +', async () => {
    const { c } = await open();
    need(tab(c, 'Queue'), 'the Queue tab').click();
    await tick();
    const [ticketed, manual] = triageRows(c);

    expect(text(ticketed)).toContain('Sync shows "synced" for a refused repository');
    const tags = all(ticketed, '[data-tag]').map(text);
    expect(tags[0]).toBe('bug');
    expect(tags).toContain('sprint 47');
    const ticket = need(one(ticketed, 'a[data-ticket]'), 'a[data-ticket]');
    expect(ticket.getAttribute('href')).toBe('https://acme.atlassian.net/browse/SEN-58');
    expect(text(ticket)).toContain('SEN-58');
    expect(one(ticketed, '[data-add-to-backlog]')?.getAttribute('aria-label')).toBe(
      'Add to backlog: Sync shows "synced" for a refused repository',
    );

    expect(all(manual, '[data-tag]').map(text)).toContain('added by you');
    expect(one(manual, 'a[data-ticket]')).toBeNull();
    expect(one(manual, '[data-add-to-backlog]')).not.toBeNull();
  });

  it.skip('pending: relay-screen — + adds a tracker task by its identity, then the re-read queue shows it in Backlog', async () => {
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

  it.skip('pending: relay-screen — + adds a manual item by its id', async () => {
    const { c, api } = await open();
    need(tab(c, 'Queue'), 'the Queue tab').click();
    await tick();
    const [, manual] = triageRows(c);
    need(one(manual, '[data-add-to-backlog]'), '[data-add-to-backlog]').click();
    await vi.waitFor(() => expect(api.tryAddToBacklog).toHaveBeenCalledTimes(1));
    expect(api.tryAddToBacklog).toHaveBeenCalledWith({ item_id: 'item-manual' });
  });

  it.skip('pending: relay-screen — a refused + says why and keeps the row in Triage', async () => {
    const { c, api } = await open();
    api.tryAddToBacklog.mockResolvedValue(F.fail(500, 'Internal Server Error'));
    need(tab(c, 'Queue'), 'the Queue tab').click();
    await tick();
    const [ticketed] = triageRows(c);
    need(one(ticketed, '[data-add-to-backlog]'), '[data-add-to-backlog]').click();

    await vi.waitFor(() => expect(text(one(c, '[data-add-error]'))).toContain('Internal Server Error'));
    expect(all(need(section(c, 'triage'), 'the triage section'), '[data-queue-row]')).toHaveLength(2);
  });

  it.skip('pending: relay-screen — Backlog is in backlog order (position), with no + and no rank numbers', async () => {
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

  it.skip('pending: relay-screen — a failed queue read is its error on Queue, never an empty queue or a 0 count', async () => {
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

  it.skip('pending: relay-screen — the queue’s Retry re-reads the queue only, and recovers', async () => {
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

  it.skip('pending: relay-screen — a labelled toolbar action opens the compose band', async () => {
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

  it.skip('pending: relay-screen — To: the coordinator, each contributor lane, everyone; About: no particular task, or a crew task', async () => {
    const { c } = await compose();
    const to = field(c, 'to') as HTMLSelectElement;
    expect([...to.options].map((o) => o.value)).toEqual(['coordinator', F.LANE.L1, 'everyone']);
    expect([...to.options].map((o) => o.textContent?.trim())[1]).toContain('L1 · Diagrams');
    const about = field(c, 'about') as HTMLSelectElement;
    expect([...about.options].map((o) => o.value)).toEqual(['', 't-review-er', 't-structure']);
    expect(about.options[0].textContent?.trim()).toBe('No particular task');
  });

  it.skip('pending: relay-screen — Queue message sends To, About and the text for this crew, then says it is queued', async () => {
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

  it.skip('pending: relay-screen — to the coordinator or everyone, about no particular task', async () => {
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

  it.skip('pending: relay-screen — an empty message, or one over 600 characters, cannot be queued', async () => {
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

  it.skip('pending: relay-screen — a refused message says why and keeps the text', async () => {
    const { c, api } = await compose();
    api.trySendMessage.mockResolvedValue(F.fail(503, 'daemon busy'));
    setValue(field(c, 'text'), 'Keep the shortcut.');
    await tick();
    need(field(c, 'send'), 'Queue message').click();

    await vi.waitFor(() => expect(text(one(c, '[data-compose-error]'))).toContain('daemon busy'));
    expect(one(c, '[data-compose-queued]')).toBeNull();
    expect((field(c, 'text') as HTMLTextAreaElement).value).toBe('Keep the shortcut.');
  });

  it.skip('pending: relay-screen — two presses of Queue message send one message', async () => {
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

  it.skip('pending: relay-screen — is the fault Notice with its code, never "No crew yet" and never empty lanes', async () => {
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

  it.skip('pending: relay-screen — Retry reads once for this workspace, and the lanes replace the notice', async () => {
    const { c, api } = await open({ now: down(), workspace: 'w2' });
    api.tryGetNow.mockClear();
    api.tryGetNow.mockResolvedValue(F.ok(F.now()));

    need(one(c, '[data-relay-error] [data-action="retry"]'), '[data-relay-error] [data-action="retry"]').click();
    await vi.waitFor(() => expect(section(c, 'lanes')).not.toBeNull());
    expect(one(c, '[data-relay-error]')).toBeNull();
    expect(api.tryGetNow).toHaveBeenCalledTimes(1);
    expect(api.tryGetNow).toHaveBeenCalledWith({ workspace_id: 'w2' });
  });

  it.skip('pending: relay-screen — a Retry that fails again shows the new reason, not the old one, and not an empty crew', async () => {
    const { c, api } = await open({ now: down() });
    api.tryGetNow.mockResolvedValue(F.fail(502, 'Bad Gateway'));

    need(one(c, '[data-relay-error] [data-action="retry"]'), '[data-relay-error] [data-action="retry"]').click();
    await vi.waitFor(() => expect(text(one(c, '[data-relay-error]'))).toContain('Bad Gateway'));
    expect(text(one(c, '[data-relay-error]'))).not.toContain('connection refused');
    expect(section(c, 'lanes')).toBeNull();
    expect(text(c)).not.toContain('No crew yet');
  });

  it.skip('pending: relay-screen — two presses of Retry are one read', async () => {
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

  it.skip('pending: relay-screen — while the Retry reads, the body shows the loading skeleton', async () => {
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
  it.skip('pending: relay-screen — the loader’s new answer replaces the old crew', async () => {
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

describe('Relay — no crew yet', () => {
  it.skip('pending: relay-screen — the info band frames it: the two paste lines, and "Listening · 0 of 2 to start"; the body is empty', async () => {
    const { c } = await open({ now: F.ok(F.noCrew()) });
    const info = need(one(c, '[data-screen-info]'), '[data-screen-info]');
    expect(info).not.toBeNull();
    expect(text(info)).toContain('No crew yet');
    expect(all(info, '[data-paste-line]').map((l) => text(one(l, '[data-paste-text]')))).toEqual([
      'register with sensei as coordinator',
      'register with sensei as contributor',
    ]);
    expect(text(info)).toContain('Listening · 0 of 2 to start');

    expect(section(c, 'needs-you')).toBeNull();
    expect(section(c, 'lanes')).toBeNull();
    expect(section(c, 'completed')).toBeNull();
    expect(one(c, '[data-relay-error]')).toBeNull();
    expect(toolbarButton(c, 'Message the crew')).toBeNull();
  });

  it.skip('pending: relay-screen — Copy puts its line on the clipboard', async () => {
    const writeText = vi.fn(async () => {});
    Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true });
    const { c } = await open({ now: F.ok(F.noCrew()) });
    const lines = all(c, '[data-paste-line]');
    expect(lines, 'the two paste lines').toHaveLength(2);
    const [, contributor] = lines;
    need(one(contributor, '[data-paste-copy]'), '[data-paste-copy]').click();
    await vi.waitFor(() => expect(writeText).toHaveBeenCalledWith('register with sensei as contributor'));
  });

  it.skip('pending: relay-screen — with no crew the info band cannot be closed', async () => {
    const { c } = await open({ now: F.ok(F.noCrew()) });
    expect(one(c, '[data-screen-info]')).not.toBeNull();
    expect(toolbarButton(c, 'What it’s for')).toBeNull();
  });
});

// ─── The shell (D87) ────────────────────────────────────────────────────────

describe('Relay — the shell (D87)', () => {
  it.skip('pending: relay-screen — rokkit’s shell for workspace.relay: mark 継, title Relay, no stats band', async () => {
    const { c } = await open();
    expect(vi.mocked(ScreenShell)).toHaveBeenCalled();
    const props = shellProps();
    expect(props).toMatchObject({ viewId: 'workspace.relay', title: 'Relay', mark: { kanji: '継' } });
    expect(props.stats).toBeUndefined();
    expect(one(c, '[data-screen-shell]')?.getAttribute('data-view')).toBe('workspace.relay');
    expect(text(one(c, '[data-screen-title]'))).toBe('Relay');
    expect(one(c, '[data-screen-stats]')).toBeNull();
  });

  it.skip('pending: relay-screen — the context names this crew by the end of its id (a uuid v7 starts with the clock)', async () => {
    const { c } = await open({
      workspace: 'w2',
      now: F.ok(F.now({ crews: [F.crew({ id: F.OTHER_CREW_ID, workspace_id: 'w2' })] })),
    });
    const context = text(one(c, '[data-screen-context]'));
    expect(context).toContain(`crew ${F.OTHER_CREW_ID.slice(-4)}`);
    expect(context).not.toContain(F.CREW_ID.slice(-4));
  });

  it.skip('pending: relay-screen — the info band with a crew: what Relay is, three steps, and Today with first-pass approval', async () => {
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

  it.skip('pending: relay-screen — first-pass approval nobody reported reads "not reported", never "0 of"', async () => {
    const { c } = await open({
      now: F.ok(F.now({ today: { completed: 2, reviewed: null, first_pass_approved: null } })),
    });
    const info = text(one(c, '[data-screen-info]'));
    expect(info).toContain('not reported');
    expect(info).not.toMatch(/0 of/);
  });

  it.skip('pending: relay-screen — the marks follow the glyph mode: kanji, or the vocabulary’s icon', async () => {
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

  it.skip('pending: relay-screen — the e2e anchor: data-screen="relay" around the shell', async () => {
    const { c } = await open();
    const anchor = one(c, '[data-screen="relay"]');
    expect(anchor).not.toBeNull();
    expect(one(anchor!, '[data-screen-shell]')).not.toBeNull();
  });
});

// ─── Footer: resources (lanes.md §8) ────────────────────────────────────────

describe('Relay — the footer resource line', () => {
  it.skip('pending: relay-screen — the build lease holder and its time, disk free, and swap free against its floor', async () => {
    const { c } = await open();
    const footer = text(one(c, '[data-screen-footer]'));
    expect(footer).toContain('Rust build');
    expect(footer).toContain('L1');
    expect(footer).toContain('4 min');
    expect(footer).toContain('Disk 46 GB free');
    expect(footer).toContain('Swap 1.2 GB free (floor 1 GB)');
  });

  it.skip('pending: relay-screen — unreported disk or swap reads "not reported"; no lease reads as no build', async () => {
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
  it.skip('pending: relay-screen — a lane row: mark, lane and state on line 1; the task and its time on line 2', async () => {
    const { c } = await open();
    const row = need(one(c, '[data-lane-row="L1"]'), '[data-lane-row="L1"]');
    const line1 = need(one(row, '[data-row-line="1"]'), '[data-row-line="1"]');
    const line2 = need(one(row, '[data-row-line="2"]'), '[data-row-line="2"]');
    expect(line1).not.toBeNull();
    expect(line2).not.toBeNull();
    expect(one(line1, '[data-lane-role]')).not.toBeNull();
    expect(text(line1)).toContain('Diagrams');
    expect(one(line1, '[data-state]')).not.toBeNull();
    expect(text(line2)).toContain('Structure view on the shell');
    expect(text(line2)).toContain('38 min');
  });

  it.skip('pending: relay-screen — a queue row keeps its buttons beside the title; the ticket button can drop the provider', async () => {
    const { c } = await open();
    need(tab(c, 'Queue'), 'the Queue tab').click();
    await tick();
    const [row] = triageRows(c);
    const line1 = need(one(row, '[data-row-line="1"]'), '[data-row-line="1"]');
    expect(one(line1, '[data-queue-title]')).not.toBeNull();
    expect(one(line1, 'a[data-ticket]')).not.toBeNull();
    expect(one(line1, '[data-add-to-backlog]')).not.toBeNull();
    const provider = need(one(row, '[data-ticket-provider]'), '[data-ticket-provider]');
    expect(text(provider)).toBe('Jira');
    expect(provider.className).toMatch(/(^|\s)(max-sm:hidden|hidden sm:inline(-flex)?)(\s|$)/);
  });
});

// ─── r2 (review relay-screen-tests-r1) ──────────────────────────────────────

/** A local "HH:MM" for an instant, as the reader's clock shows it. */
const hhmm = (iso: string) => {
  const d = new Date(iso);
  return `${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`;
};

describe('Relay r2 — the header (F1, F2, L4)', () => {
  it.skip('pending: relay-screen — search is off on Relay (L0 ruling): no search field, no filters band', async () => {
    const { c } = await open();
    expect(vi.mocked(ScreenShell)).toHaveBeenCalled();
    expect(shellProps().search).toBe(false);
    expect(one(c, '[data-screen-search]')).toBeNull();
    expect(one(c, '[data-screen-filters]')).toBeNull();
  });

  it.skip('pending: relay-screen — the header’s controls are exactly Now, Queue, Message the crew and the info toggle', async () => {
    const { c } = await open();
    const header = need(one(c, '[data-screen-header]'), 'the shell header');
    const controls = all(header, 'button, a, [role="tab"]')
      .map((el) => el.getAttribute('aria-label') || text(el))
      .map((label) => (label.startsWith('Queue') ? 'Queue' : label))
      .sort();
    // No stats / filters / side toggles, and no other action (CONTRACT §1a, user r1).
    expect(controls).toEqual(['Message the crew', 'Now', 'Queue', 'What it’s for']);
  });

  it.skip('pending: relay-screen — the context is the workspace, then the crew', async () => {
    const { c } = await open({
      workspace: 'w2',
      layout: { workspace: { id: 'w2', name: 'Billing service' }, workspaceId: 'w2' },
      now: F.ok(F.now({ crews: [F.crew({ id: F.OTHER_CREW_ID, workspace_id: 'w2' })] })),
    });
    expect(text(one(c, '[data-screen-context]'))).toBe(`Billing service · crew ${F.OTHER_CREW_ID.slice(-4)}`);
  });
});

describe('Relay r2 — one write per press (F4)', () => {
  it.skip('pending: relay-screen — two presses of Answer answer once', async () => {
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
    await vi.waitFor(() => expect(one(c, '[data-decision]')).toBeNull());
    expect(api.tryAnswerDecision).toHaveBeenCalledTimes(1);
  });

  it.skip('pending: relay-screen — two presses of + add once', async () => {
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
  it.skip('pending: relay-screen — To lists every contributor lane in the view’s order; About skips the idle lane', async () => {
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
    needs_answer: false,
    ...over,
  });

  it.skip('pending: relay-screen — blocked on a dependency reads "blocked"; only an open decision reads "needs your answer"', async () => {
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
    expect(text(one(c, '[data-lane-row="L4"] [data-state]'))).toBe('blocked');
    expect(text(one(c, '[data-lane-row="L3"] [data-state]'))).toBe('needs your answer');
  });

  it.skip('pending: relay-screen — the lane lease drives "lease lost", with the time it expired; a task interrupted under a live lease reads "interrupted"', async () => {
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
    expect(text(one(lost, '[data-state]'))).toBe('lease lost');
    expect(text(lost)).toContain(`at ${hhmm(expired)}`);
    expect(text(one(c, '[data-lane-row="L5"] [data-state]'))).toBe('interrupted');
  });
});

describe('Relay r2 — Completed today rows (L1)', () => {
  it.skip('pending: relay-screen — each row leads with its time, and "done" (nothing to merge) has its own label', async () => {
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
    expect(text(one(rows[1], '[data-state]'))).toBe('done');
  });
});
