// @vitest-environment jsdom
// Observatory · Relay — the main window's Relay (D90; repository ruling, user 2026-11-05).
//
// 67 of 157 repositories here belong to no workspace, so Relay lives in the main window and
// reads every crew; it filters by repository or workspace. The workspace window shows the same
// screen pre-filtered to its workspace (`(workspace)/workspace/[id]/relay`, whose spec pins
// Now, Queue, compose and the states). This file pins what differs in the main window.
//
// The loader and the page are imported inside each test. The relay client is mocked.
import { describe, it, expect, afterEach, vi } from 'vitest';
import { tick } from 'svelte';
import { mountComponent } from '$lib/test-mount.js';
import type { ApiResult } from '$lib/api.js';
import type { RelayNow, RelayQueue } from '$lib/relay-api.js';
import * as F from '$lib/test-stubs/relay-fixtures.js';

const PAGE = './+page.svelte';
const LOADER = './+page.js';

const relay = vi.hoisted(() => ({ api: null as unknown }));
vi.mock('$lib/relay-api.js', () => ({ relayApi: vi.fn(() => relay.api) }));
// The socket is the route's, pinned in relay-crew-stream.spec — not here.
vi.mock('$lib/relay/relay-crew-stream.js', () => ({ subscribeCrewStream: vi.fn(() => () => {}) }));

const route = vi.hoisted(() => ({ search: '' }));
vi.mock('$app/state', () => ({
  page: {
    params: {},
    get url() {
      return new URL(`http://localhost/relay${route.search}`);
    },
  },
}));
vi.mock('$app/navigation', () => ({ goto: vi.fn(), invalidateAll: vi.fn() }));

vi.mock('@rokkit/app', async (importOriginal) => {
  const m = (await importOriginal()) as Record<string, unknown>;
  return { ...m, ScreenShell: vi.fn(m.ScreenShell as never) };
});

import { goto } from '$app/navigation';
import { ScreenShell } from '@rokkit/app';

let cleanup: Array<() => void> = [];
afterEach(() => {
  cleanup.forEach((fn) => fn());
  cleanup = [];
  vi.unstubAllGlobals();
  vi.mocked(ScreenShell).mockClear();
  vi.mocked(goto).mockClear();
  route.search = '';
});

async function open(opts: { now?: ApiResult<RelayNow>; queue?: ApiResult<RelayQueue>; search?: string } = {}) {
  const api = F.fakeRelayApi();
  api.tryGetNow.mockResolvedValue(opts.now ?? F.ok(F.mainNow()));
  if (opts.queue) api.tryGetQueue.mockResolvedValue(opts.queue);
  relay.api = api;
  route.search = opts.search ?? '';
  const { load } = await import(/* @vite-ignore */ LOADER);
  const data = await load({ params: {}, url: new URL(`http://localhost/relay${route.search}`) } as never);
  const { default: Page } = await import(/* @vite-ignore */ PAGE);
  const m = mountComponent(Page, { data } as never);
  cleanup.push(m.destroy);
  await tick();
  return { c: m.container, api };
}

const one = (root: ParentNode, sel: string) => root.querySelector(sel) as HTMLElement | null;
const all = (root: ParentNode, sel: string) => [...root.querySelectorAll(sel)] as HTMLElement[];
const text = (el: Element | null | undefined) => (el?.textContent ?? '').replace(/\s+/g, ' ').trim();

/** The element, or a failure that names what is missing — never a null dereference. */
function need<T extends Element>(el: T | null | undefined, what: string): T {
  expect(el ?? null, `missing: ${what}`).not.toBeNull();
  return el as T;
}

function setValue(el: HTMLElement, value: string, event = 'change') {
  (el as HTMLSelectElement).value = value;
  el.dispatchEvent(new Event(event, { bubbles: true }));
}

const shellProps = () => vi.mocked(ScreenShell).mock.calls[0][1] as unknown as Record<string, unknown>;

describe('Observatory · Relay — the loader’s read', () => {
  it('with no filter it reads every crew, and the page reads nothing of its own', async () => {
    const fetchSpy = vi.fn();
    vi.stubGlobal('fetch', fetchSpy);
    const { api } = await open();
    expect(api.tryGetNow).toHaveBeenCalledTimes(1);
    expect(api.tryGetNow).toHaveBeenCalledWith({});
    expect(api.tryGetQueue).toHaveBeenCalledWith({});
    expect(fetchSpy).not.toHaveBeenCalled();
  });

  it('the URL’s repository or workspace narrows the read', async () => {
    const byRepo = await open({ search: `?repo=${encodeURIComponent(F.LOOSE_REPO)}` });
    expect(byRepo.api.tryGetNow).toHaveBeenCalledWith({ repo_key: F.LOOSE_REPO });
    expect(byRepo.api.tryGetQueue).toHaveBeenCalledWith({ repo_key: F.LOOSE_REPO });

    const byWorkspace = await open({ search: '?workspace=w1' });
    expect(byWorkspace.api.tryGetNow).toHaveBeenCalledWith({ workspace_id: 'w1' });
  });
});

describe('Observatory · Relay — every crew, workspace or not', () => {
  it('a task on a repository with no workspace appears, with its repository, and opens by its crew', async () => {
    const { c } = await open();
    const lanes = need(one(c, '[data-relay-section="lanes"]'), 'the lanes section');
    expect(all(lanes, '[data-lane-row]').map((r) => r.getAttribute('data-lane-row'))).toEqual(['L0', 'L1', 'L8', 'L9']);
    const loose = need(one(lanes, '[data-lane-row="L9"]'), 'the L9 row');
    expect(text(loose)).toContain('Prototype the scratch parser');
    const looseGroup = need(one(c, `[data-crew-group="${F.LOOSE_CREW_ID}"]`), 'the loose crew’s group');
    expect(text(one(looseGroup, '[data-crew-heading]'))).toContain('jerrythomas/scratch');
    expect(one(loose, `a[href="/relay/${F.LOOSE_CREW_ID}/t-scratch"]`)).not.toBeNull();
    expect(one(c, `[data-lane-row="L1"] a[href="/relay/${F.CREW_ID}/t-structure"]`)).not.toBeNull();
  });

  it('lane rows are grouped by crew, and each group’s heading names its repository', async () => {
    const { c } = await open();
    const groups = all(c, '[data-crew-group]');
    expect(groups.map((g) => g.getAttribute('data-crew-group'))).toEqual([F.CREW_ID, F.LOOSE_CREW_ID]);

    const sensei = need(one(c, `[data-crew-group="${F.CREW_ID}"]`), 'the workspace crew’s group');
    expect(text(one(sensei, '[data-crew-heading]'))).toContain('github.com/sensei-hq/sensei');
    expect(all(sensei, '[data-lane-row]').map((r) => r.getAttribute('data-lane-row'))).toEqual(['L0', 'L1']);

    const loose = need(one(c, `[data-crew-group="${F.LOOSE_CREW_ID}"]`), 'the loose crew’s group');
    expect(text(one(loose, '[data-crew-heading]'))).toContain('github.com/jerrythomas/scratch');
    expect(all(loose, '[data-lane-row]').map((r) => r.getAttribute('data-lane-row'))).toEqual(['L8', 'L9']);
  });

  it('two crews with no repository head distinct "no repository · crew XXXX" groups (Q18 L1)', async () => {
    const base = F.mainNow();
    const { c } = await open({
      now: F.ok({
        ...base,
        crews: [
          F.crew({ repo_key: null }),
          F.crew({ id: F.LOOSE_CREW_ID, repo_key: null, workspace_id: null, goal: 'Try the scratch idea' }),
        ],
      }),
    });
    const sensei = need(one(c, `[data-crew-group="${F.CREW_ID}"]`), 'the first repo-less crew’s group');
    expect(text(one(sensei, '[data-crew-heading]'))).toBe('no repository · crew c7f3');
    const loose = need(one(c, `[data-crew-group="${F.LOOSE_CREW_ID}"]`), 'the second repo-less crew’s group');
    expect(text(one(loose, '[data-crew-heading]'))).toBe('no repository · crew b10c');
  });

  // #394 asked for the crew's size instead of a crew count. The r1 mockup (CONTRACT
  // §1a) puts the size on the Crew button's badge and makes the context name the
  // place and the run — the main window, which may hold several crews, counts them.
  it('the crew’s size is the Crew button’s count; the context names the place and the crews (#394, r1)', async () => {
    const { c } = await open();
    const crew = need(one(c, '[data-screen-tools] [aria-label^="Crew"]'), 'the Crew button');
    expect(crew.getAttribute('aria-label')).toBe('Crew · 4');
    expect(text(one(c, '[data-screen-context]'))).toBe('all workspaces · 2 crews');
  });

  it('one crew: the context names it by the end of its id; one assistant is a count of 1', async () => {
    const { c } = await open({ now: F.ok(F.now({ lanes: [F.coordinatorLane()] })) });
    expect(text(one(c, '[data-screen-context]'))).toBe(`all workspaces · crew ${F.CREW_ID.slice(-4)}`);
    expect(need(one(c, '[data-screen-tools] [aria-label^="Crew"]'), 'the Crew button').getAttribute('aria-label')).toBe('Crew · 1');
  });

  it('a failed read says the crew was not reported, never "0" or "no run" (D83)', async () => {
    const { c } = await open({ now: F.fail(503, 'Service Unavailable') });
    expect(text(one(c, '[data-screen-context]'))).toBe('all workspaces · crew not reported');
    expect(one(c, '[data-screen-tools] [aria-label^="Crew"]')).toBeNull();
  });
});

describe('Observatory · Relay — the shell (D87)', () => {
  it('rokkit’s shell for observatory.relay: mark 継, title Relay, search off, the four stats on Now', async () => {
    const { c } = await open();
    expect(vi.mocked(ScreenShell)).toHaveBeenCalled();
    const props = shellProps();
    expect(props).toMatchObject({ viewId: 'observatory.relay', title: 'Relay', mark: { kanji: '継' } });
    expect(props.search).toBe(false);
    // CONTRACT §1c-0: Working · Waiting · Blocked · Completed today, each a filter of Now.
    expect((props.stats as Array<{ id: string }>).map((s) => s.id)).toEqual(['working', 'waiting', 'blocked', 'done']);
    expect(one(c, '[data-screen-search]')).toBeNull();
    const anchor = need(one(c, '[data-screen="relay"]'), 'the e2e anchor');
    expect(anchor.hasAttribute('data-screen-shell'), 'the anchor is the shell itself').toBe(true);
  });

  it('the header’s controls: the three views, Message the crew and the info toggle — no filters toggle, the band stays', async () => {
    const { c } = await open();
    const header = need(one(c, '[data-screen-header]'), 'the shell header');
    // The stats band and the filter band live in the header too; their controls are theirs.
    const controls = all(header, 'button, a, [role="tab"]')
      .filter((el) => !el.closest('[data-screen-filters]') && !el.closest('[data-screen-stats]'))
      .map((el) => el.getAttribute('aria-label') || text(el))
      .map((label) => (label.startsWith('Queue') || label.startsWith('Crew') ? label.split(' · ')[0] : label))
      .sort();
    expect(controls).toEqual(['Crew', 'Hide info', 'Message the crew', 'Now', 'Queue']);
    expect(one(c, '[data-screen-filters]'), 'the repository/workspace band').not.toBeNull();
  });
});

describe('Observatory · Relay — the filter band: repository and workspace (#384)', () => {
  type Element = { scope: string; label: string; props: { options: Array<{ value: string; label: string }> } };
  const filterShell = () => {
    const props = shellProps();
    return {
      schema: (props.filters as { schema: Record<string, unknown> }).schema,
      elements: (props.filters as { layout: { elements: Element[] } }).layout.elements,
      filter: props.filter as { values: Record<string, unknown>; stat: string | null },
      onfilter: props.onfilter as (next: { values: Record<string, unknown>; stat: string | null }) => void,
    };
  };

  it('a rokkit schema form lists the repositories and workspaces the crews work in', async () => {
    const { c } = await open({ search: '?workspace=w1' });
    const { schema, elements, filter } = filterShell();
    expect(Object.keys((schema as { properties: object }).properties)).toEqual(['repo', 'workspace']);
    expect(elements.map((e) => e.props.options.map((o) => o.value))).toEqual([['', F.REPO, F.LOOSE_REPO], ['', 'w1']]);
    expect(elements[1].props.options[1].label).toBe('Sensei');
    // The URL's narrowing is the form's value.
    expect(filter.values).toMatchObject({ repo: '', workspace: 'w1' });
    const band = need(one(c, '[data-screen-filters]'), 'the filter band');
    need(one(band, '[data-form-root]'), 'the rokkit form');
    expect(text(band)).toContain('Repository');
    expect(text(band)).toContain('Workspace');
    expect(one(band, 'select[data-relay-filter-repo]'), 'no hand-made select').toBeNull();
  });

  it('choosing one navigates with it in the URL; All clears it', async () => {
    const { api } = await open({ search: '?workspace=w1' });
    filterShell().onfilter({ values: { repo: F.LOOSE_REPO, workspace: 'w1' }, stat: null });
    await tick();
    expect(vi.mocked(goto)).toHaveBeenCalledTimes(1);
    const [href, ...rest] = vi.mocked(goto).mock.calls[0];
    const url = new URL(String(href), 'http://localhost');
    expect(url.pathname).toBe('/relay');
    expect(url.searchParams.get('repo')).toBe(F.LOOSE_REPO);
    expect(url.searchParams.get('workspace')).toBe('w1');
    // N9: goto re-runs this route's load on a search change, and nothing above it
    // depends on the filter — invalidateAll would refetch every parent load too.
    expect(rest).toEqual([]);
    // One load for the narrow: the page fetches nothing of its own.
    expect(api.tryGetNow).toHaveBeenCalledTimes(1);
    expect(api.tryGetQueue).toHaveBeenCalledTimes(1);

    filterShell().onfilter({ values: { repo: '', workspace: '' }, stat: null });
    await tick();
    const cleared = new URL(String(vi.mocked(goto).mock.calls[1][0]), 'http://localhost');
    expect(cleared.searchParams.has('workspace')).toBe(false);
  });
});

describe('Observatory · Relay — Message the crew across crews', () => {
  it('with two crews the person picks the crew; To and About are that crew’s; the message is for it', async () => {
    const { c, api } = await open();
    need(one(c, '[data-screen-tools] [aria-label="Message the crew"]'), 'the Message the crew action').click();
    await tick();
    const crewField = need(one(c, 'select[data-compose-crew]'), 'the Crew field') as HTMLSelectElement;
    expect([...crewField.options].map((o) => o.value)).toEqual([F.CREW_ID, F.LOOSE_CREW_ID]);

    setValue(crewField, F.LOOSE_CREW_ID);
    await tick();
    const to = need(one(c, 'select[data-compose-to]'), 'To') as HTMLSelectElement;
    expect([...to.options].map((o) => o.value)).toEqual(['coordinator', F.LANE.L9, 'everyone']);
    const about = need(one(c, 'select[data-compose-about]'), 'About') as HTMLSelectElement;
    expect([...about.options].map((o) => o.value)).toEqual(['', 't-scratch']);

    setValue(to, F.LANE.L9);
    setValue(about, 't-scratch');
    setValue(need(one(c, '[data-compose-text]'), 'the message'), 'Try the other parser first.', 'input');
    await tick();
    need(one(c, '[data-compose-send]'), 'Queue message').click();
    await vi.waitFor(() => expect(api.trySendMessage).toHaveBeenCalledTimes(1));
    expect(api.trySendMessage).toHaveBeenCalledWith({
      crew_id: F.LOOSE_CREW_ID,
      to: { kind: 'lane', lane_id: F.LANE.L9 },
      task_id: 't-scratch',
      text: 'Try the other parser first.',
      surface: 'desktop',
    });
  });

  it('repo-less crews read distinct "no repository · crew XXXX" in the Crew field (Q18 L1)', async () => {
    const base = F.mainNow();
    const { c } = await open({
      now: F.ok({
        ...base,
        crews: [
          F.crew(),
          F.crew({ id: F.LOOSE_CREW_ID, repo_key: null, workspace_id: null, goal: 'Try the scratch idea' }),
          F.crew({ id: F.OTHER_CREW_ID, repo_key: null, workspace_id: null, goal: 'Another scratch' }),
        ],
      }),
    });
    need(one(c, '[data-screen-tools] [aria-label="Message the crew"]'), 'the Message the crew action').click();
    await tick();
    const crewField = need(one(c, 'select[data-compose-crew]'), 'the Crew field') as HTMLSelectElement;
    expect([...crewField.options].map((o) => o.text)).toEqual([
      'github.com/sensei-hq/sensei',
      'no repository · crew b10c',
      'no repository · crew 99aa',
    ]);
  });
});
