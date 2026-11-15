// @vitest-environment jsdom
// Workspace · Relay · a past run, read-only (CONTRACT §1i; #424, the UI half of #393):
// the banner, who was on it, what it did — each task opening its task view.
import { describe, it, expect, afterEach, beforeAll, vi } from 'vitest';
import { tick } from 'svelte';
import { mountComponent } from '$lib/test-mount.js';
import type { ApiResult } from '$lib/api.js';
import type { RelayRunDetail } from '@sensei/state';
import * as F from '$lib/test-stubs/relay-fixtures.js';

const PAGE = './+page.svelte';
const LOADER = './+page.js';

const relay = vi.hoisted(() => ({ api: null as unknown }));
vi.mock('$lib/relay-api.js', () => ({ relayApi: vi.fn(() => relay.api) }));
const nav = vi.hoisted(() => ({ invalidateAll: vi.fn(async () => {}) }));
vi.mock('$app/navigation', () => ({ goto: vi.fn(), invalidateAll: nav.invalidateAll }));
vi.mock('$app/state', () => ({
  page: {
    params: { id: 'w1', run: '' },
    url: new URL('http://localhost/workspace/w1/relay/runs/x'),
  },
}));

// The page and its loader compile on first import; under the full suite that alone can
// pass a test's 5 s, so it is paid once here, not inside the first test.
beforeAll(async () => {
  await import(/* @vite-ignore */ PAGE);
  await import(/* @vite-ignore */ LOADER);
}, 30_000);

let cleanup: Array<() => void> = [];
afterEach(() => {
  cleanup.forEach((fn) => fn());
  cleanup = [];
  nav.invalidateAll.mockClear();
});

async function open(run: ApiResult<RelayRunDetail> = F.ok(F.runDetail())) {
  const api = F.fakeRelayApi();
  api.tryGetRun.mockResolvedValue(run);
  relay.api = api;
  const { load } = await import(/* @vite-ignore */ LOADER);
  const data = await load({ params: { id: 'w1', run: F.PAST_RUN_ID } } as never);
  const { default: Page } = await import(/* @vite-ignore */ PAGE);
  const m = mountComponent(Page, { data } as never);
  cleanup.push(m.destroy);
  await tick();
  return { c: m.container, api };
}

const one = (root: ParentNode, sel: string) => root.querySelector(sel) as HTMLElement | null;
const all = (root: ParentNode, sel: string) => [...root.querySelectorAll(sel)] as HTMLElement[];
const text = (el: Element | null | undefined) => (el?.textContent ?? '').replace(/\s+/g, ' ').trim();
function need<T extends Element>(el: T | null | undefined, what: string): T {
  expect(el ?? null, `missing: ${what}`).not.toBeNull();
  return el as T;
}

describe('Relay · a past run (read-only)', () => {
  it('reads the run by its id, once', async () => {
    const { api } = await open();
    expect(api.tryGetRun).toHaveBeenCalledTimes(1);
    expect(api.tryGetRun).toHaveBeenCalledWith(F.PAST_RUN_ID);
  });

  it('the run, its banner and facts; who was on it; what it did, each task opening its view; back to Relay', async () => {
    const { c } = await open();
    const run = need(one(c, '[data-archived-run]'), 'the past run');
    expect(text(one(run, '[data-run-title]'))).toContain(`Run ${F.PAST_RUN_ID.slice(-4)}`);
    expect(text(one(run, '[data-run-banner]'))).toContain('Archived when its last task was done');
    expect(all(run, '[data-run-fact]').map(text)).toEqual(['Tasks 3', 'Done 2', 'Members 2']);
    expect(all(run, '[data-run-member]').map((m) => m.getAttribute('data-run-member'))).toEqual(['L1', 'L2']);
    const tasks = all(run, '[data-run-task]');
    expect(tasks.map((t) => t.getAttribute('data-run-task'))).toEqual(['t-plan', 't-header', 't-extra']);
    expect(tasks[0].closest('a')?.getAttribute('href')).toBe(`/workspace/w1/relay/${F.CREW_ID}/t-plan`);
    expect(one(c, '[data-back]')?.getAttribute('href')).toBe('/workspace/w1/relay');
    // Read-only: no Start, no Cancel, nothing to answer.
    expect(one(c, '[data-run-start], [data-run-cancel]')).toBeNull();
  });

  it('a run that is not there (deleted, or someone else’s) says so; no Retry', async () => {
    const { c } = await open(F.fail(404, 'run not found'));
    const notFound = need(one(c, '[data-relay-not-found]'), 'the not-found notice');
    expect(text(notFound)).toContain("This run isn't here");
    expect(one(c, '[data-archived-run]')).toBeNull();
    expect(one(c, '[data-back]')?.getAttribute('href')).toBe('/workspace/w1/relay');
  });

  it('a failed read says so, with Retry reading again', async () => {
    const { c } = await open(F.fail(503, 'daemon busy'));
    const error = need(one(c, '[data-relay-error]'), 'the error notice');
    expect(text(error)).toContain("Couldn't read this run");
    need(one(error, '[data-action="retry"]'), 'Retry').click();
    await tick();
    expect(nav.invalidateAll).toHaveBeenCalledTimes(1);
  });
});
