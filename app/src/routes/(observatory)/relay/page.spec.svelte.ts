// @vitest-environment jsdom
// Observatory · Relay — the main window's Relay (D90; repository ruling, user 2026-11-05).
//
// 67 of 157 repositories here belong to no workspace, so Relay lives in the main window and
// reads every crew; it filters by repository or workspace. The workspace window shows the same
// screen pre-filtered to its workspace (`(workspace)/workspace/[id]/relay`, whose spec pins
// Now, Queue, compose and the states). This file pins what differs in the main window.
//
// Phase A: disabled reds (`pending: relay-screen`). The route does not exist yet, so the
// loader and the page are imported inside each test. The relay client is mocked.
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

  it('the context counts the crews it shows', async () => {
    const { c } = await open();
    expect(text(one(c, '[data-screen-context]'))).toContain('2 crews');
  });
});

describe('Observatory · Relay — the shell (D87)', () => {
  it('rokkit’s shell for observatory.relay: mark 継, title Relay, search off, no stats', async () => {
    const { c } = await open();
    expect(vi.mocked(ScreenShell)).toHaveBeenCalled();
    const props = shellProps();
    expect(props).toMatchObject({ viewId: 'observatory.relay', title: 'Relay', mark: { kanji: '継' } });
    expect(props.search).toBe(false);
    expect(props.stats).toBeUndefined();
    expect(one(c, '[data-screen-search]')).toBeNull();
    const anchor = need(one(c, '[data-screen="relay"]'), 'the e2e anchor');
    expect(one(anchor, '[data-screen-shell]')).not.toBeNull();
  });

  it('the header’s controls: Now, Queue, Message the crew, the info and the filters toggles', async () => {
    const { c } = await open();
    const header = need(one(c, '[data-screen-header]'), 'the shell header');
    const controls = all(header, 'button, a, [role="tab"]')
      .filter((el) => !el.closest('[data-screen-filters]'))
      .map((el) => el.getAttribute('aria-label') || text(el))
      .map((label) => (label.startsWith('Queue') ? 'Queue' : label))
      .sort();
    expect(controls).toEqual(['Filters', 'Message the crew', 'Now', 'Queue', 'What it’s for']);
  });
});

describe('Observatory · Relay — the filter band: repository and workspace', () => {
  it('lists the repositories and workspaces the crews work in', async () => {
    const { c } = await open();
    const band = need(one(c, '[data-screen-filters]'), 'the filter band');
    const repo = need(one(band, 'select[data-relay-filter-repo]'), 'the repository filter') as HTMLSelectElement;
    expect([...repo.options].map((o) => o.value)).toEqual(['', F.REPO, F.LOOSE_REPO]);
    const ws = need(one(band, 'select[data-relay-filter-workspace]'), 'the workspace filter') as HTMLSelectElement;
    expect([...ws.options].map((o) => o.value)).toEqual(['', 'w1']);
    expect(ws.options[1].textContent?.trim()).toBe('Sensei');
  });

  it('choosing one navigates with it in the URL; All clears it', async () => {
    const { c } = await open({ search: '?workspace=w1' });
    const band = need(one(c, '[data-screen-filters]'), 'the filter band');
    setValue(need(one(band, 'select[data-relay-filter-repo]'), 'the repository filter'), F.LOOSE_REPO);
    await tick();
    expect(vi.mocked(goto)).toHaveBeenCalledTimes(1);
    const [href, opts] = vi.mocked(goto).mock.calls[0];
    const url = new URL(String(href), 'http://localhost');
    expect(url.pathname).toBe('/relay');
    expect(url.searchParams.get('repo')).toBe(F.LOOSE_REPO);
    expect(url.searchParams.get('workspace')).toBe('w1');
    expect(opts).toMatchObject({ invalidateAll: true });

    setValue(need(one(band, 'select[data-relay-filter-workspace]'), 'the workspace filter'), '');
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
});
