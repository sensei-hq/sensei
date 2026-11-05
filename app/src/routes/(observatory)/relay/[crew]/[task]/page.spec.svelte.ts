// @vitest-environment jsdom
// Observatory · Relay · a task — the main window's task view (repository ruling).
//
// The same task view as the workspace window (whose spec pins its content); this file pins
// what differs here: the view id, the back link to the main Relay, the read by (crew, task),
// and a task on a repository that belongs to no workspace.
//
// Phase A: disabled reds (`pending: relay-screen`); the route is imported inside each test.
import { describe, it, expect, afterEach, vi } from 'vitest';
import { tick } from 'svelte';
import { mountComponent } from '$lib/test-mount.js';
import type { ApiResult } from '$lib/api.js';
import type { RelayTaskView } from '$lib/relay-api.js';
import * as F from '$lib/test-stubs/relay-fixtures.js';

const PAGE = './+page.svelte';
const LOADER = './+page.js';

const relay = vi.hoisted(() => ({ api: null as unknown }));
vi.mock('$lib/relay-api.js', () => ({ relayApi: vi.fn(() => relay.api) }));

const route = vi.hoisted(() => ({ crew: '', task: '' }));
vi.mock('$app/state', () => ({
  page: {
    get params() {
      return { crew: route.crew, task: route.task };
    },
    get url() {
      return new URL(`http://localhost/relay/${route.crew}/${route.task}`);
    },
  },
}));
vi.mock('$app/navigation', () => ({ goto: vi.fn(), invalidateAll: vi.fn() }));

vi.mock('@rokkit/app', async (importOriginal) => {
  const m = (await importOriginal()) as Record<string, unknown>;
  return { ...m, ScreenShell: vi.fn(m.ScreenShell as never) };
});

import { ScreenShell } from '@rokkit/app';

let cleanup: Array<() => void> = [];
afterEach(() => {
  cleanup.forEach((fn) => fn());
  cleanup = [];
  vi.mocked(ScreenShell).mockClear();
});

/** A task on a repository that belongs to no workspace. */
const looseTask = (): RelayTaskView => {
  const view = F.taskView();
  return {
    ...view,
    task: {
      ...view.task,
      id: 't-scratch',
      crew_id: F.LOOSE_CREW_ID,
      title: 'Prototype the scratch parser',
      repo_key: F.LOOSE_REPO,
      workspace_id: null,
      ticket: null,
    },
  };
};

async function open(crewId: string, taskId: string, answer: ApiResult<RelayTaskView>) {
  const api = F.fakeRelayApi();
  api.tryGetTask.mockResolvedValue(answer);
  relay.api = api;
  route.crew = crewId;
  route.task = taskId;
  const { load } = await import(/* @vite-ignore */ LOADER);
  const data = await load({
    params: { crew: crewId, task: taskId },
    url: new URL(`http://localhost/relay/${crewId}/${taskId}`),
  } as never);
  const { default: Page } = await import(/* @vite-ignore */ PAGE);
  const m = mountComponent(Page, { data } as never);
  cleanup.push(m.destroy);
  await tick();
  return { c: m.container, api };
}

const one = (root: ParentNode, sel: string) => root.querySelector(sel) as HTMLElement | null;
const text = (el: Element | null | undefined) => (el?.textContent ?? '').replace(/\s+/g, ' ').trim();

describe('Observatory · Relay · task', () => {
  it.skip('pending: relay-screen — reads by (crew, task), on observatory.relay.task, back to the main Relay', async () => {
    const { c, api } = await open(F.LOOSE_CREW_ID, 't-scratch', F.ok(looseTask()));
    expect(api.tryGetTask).toHaveBeenCalledTimes(1);
    expect(api.tryGetTask).toHaveBeenCalledWith(F.LOOSE_CREW_ID, 't-scratch');
    expect(vi.mocked(ScreenShell)).toHaveBeenCalled();
    expect(vi.mocked(ScreenShell).mock.calls[0][1]).toMatchObject({ viewId: 'observatory.relay.task', mark: { kanji: '継' } });
    expect(one(c, 'a[data-back]')?.getAttribute('href')).toBe('/relay');
  });

  it.skip('pending: relay-screen — a task on a repository with no workspace draws, naming its repository', async () => {
    const { c } = await open(F.LOOSE_CREW_ID, 't-scratch', F.ok(looseTask()));
    expect(text(one(c, '[data-task-title]'))).toBe('Prototype the scratch parser');
    expect(text(one(c, '[data-task-facts]'))).toContain('jerrythomas/scratch');
    expect(text(c)).not.toMatch(/undefined|null/);
  });
});
