// @vitest-environment jsdom
// Now's Lanes section by run and by agent (#422): each crew's open runs — the band that
// heads each, its tasks waiting (by priority), in progress and done (in run order, with
// who did and reviewed them) — then the lanes on no run task; or one group per lane.
// Row data comes from the @sensei/state selectors over the fixtures; nothing is read here.
import { describe, it, expect, afterEach, vi } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import { toAgentGroups, toRunGroups, type NowRowOptions } from '@sensei/state';
import * as F from '$lib/test-stubs/relay-fixtures.js';
import LaneList from './LaneList.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => {
  cleanup.forEach((fn) => fn());
  cleanup = [];
});
function mount(props: Record<string, unknown>) {
  const m = mountComponent(LaneList as never, props as never);
  cleanup.push(m.destroy);
  return m.container;
}
const text = (el: Element | null | undefined) => (el?.textContent ?? '').replace(/\s+/g, ' ').trim();
const opts: NowRowOptions = { at: Date.now(), threshold: 30, href: (c, t) => `/relay/${c}/${t}`, byCrew: false };
const views = (now = F.runNow(), o = opts) => ({ crews: toRunGroups(now, o), agents: toAgentGroups(now, o) });
const titlesIn = (root: Element, stage: string) =>
  [...root.querySelectorAll(`[data-run-stage="${stage}"] [data-run-task-title]`)].map((e) => text(e));

describe('LaneList · by run', () => {
  it('a run is headed by its band, then waiting, in progress and done, each in its order', () => {
    const c = mount({ ...views(), view: 'run' });
    const run = c.querySelector(`[data-run-group="${F.RUN_ID}"]`)!;
    expect(run.querySelector(`[data-sprint-band][data-run-id="${F.RUN_ID}"]`)).not.toBeNull();
    expect(titlesIn(run, 'waiting')).toEqual(['Search on the shell', 'Docs for the shell']);
    expect(titlesIn(run, 'active')).toEqual(['Structure view on the shell']);
    expect(titlesIn(run, 'done')).toEqual(['Plan: Sprint 14', 'Sidebar on the shell', 'Header on the shell']);
  });

  it('a row says its priority, who did it and who reviewed it, and links to the task', () => {
    const c = mount({ ...views(), view: 'run' });
    const row = (id: string) => c.querySelector(`[data-run-task="${id}"]`)!;
    expect(text(row('t-search').querySelector('[data-run-task-priority]'))).toBe('#1 in the backlog');
    expect(text(row('t-structure').querySelector('[data-run-task-who]'))).toBe('cobalt-lyra · Claude Code 2.1 · claude-opus-5-5');
    expect(text(row('t-structure').querySelector('[data-run-task-detail]'))).toBe('Writing the failing specs');
    expect(text(row('t-a').querySelector('[data-run-task-who]'))).toBe('cobalt-lyra · Claude Code 2.1 · claude-opus-5-5 — reviewed by coral-spica');
    expect(row('t-a').closest('a')?.getAttribute('href')).toBe(`/relay/${F.CREW_ID}/t-a`);
  });

  it('the lanes on no run task follow the runs, under their own label', () => {
    const c = mount({ ...views(), view: 'run' });
    const idle = c.querySelector('[data-lanes-idle]')!;
    expect(text(idle.querySelector('[data-lanes-idle-label]'))).toBe('Not on a run task');
    expect([...idle.querySelectorAll('[data-lane-row]')].map((e) => e.getAttribute('data-lane-row'))).toEqual(['L0']);
  });

  it('with no open run the lanes are drawn alone, with no label', () => {
    const c = mount({ ...views(F.now()), view: 'run' });
    expect(c.querySelector('[data-run-group]')).toBeNull();
    expect(c.querySelector('[data-lanes-idle-label]')).toBeNull();
    expect(c.querySelectorAll('[data-lane-row]')).toHaveLength(2);
  });

  it('the heading counts the lanes', () => {
    const c = mount({ ...views(), view: 'run' });
    expect(text(c.querySelector('[data-section-heading]'))).toBe('Lanes · 2');
  });
});

describe('LaneList · by agent', () => {
  it('one group per lane: its codename, assistant and model, state, then the tasks it ran', () => {
    const c = mount({ ...views(), view: 'agent' });
    const l1 = c.querySelector(`[data-agent-group="${F.LANE.L1}"]`)!;
    expect(text(l1.querySelector('[data-agent-name]'))).toBe('cobalt-lyra');
    expect(text(l1.querySelector('[data-agent-who]'))).toBe('Claude Code 2.1 · claude-opus-5-5');
    expect([...l1.querySelectorAll('[data-run-task-title]')].map((e) => text(e))).toEqual(['Header on the shell', 'Structure view on the shell']);
    expect(c.querySelector('[data-run-group]')).toBeNull();
  });

  it('a lane that ran nothing says so', () => {
    const now = F.runNow();
    now.run_tasks = [];
    const c = mount({ ...views(now), view: 'agent' });
    expect(text(c.querySelector(`[data-agent-group="${F.LANE.L0}"] [data-agent-empty]`))).toBe('No run tasks yet');
  });
});

describe('LaneList · the switch', () => {
  it('By run | By agent, the current one pressed; a choice calls back', () => {
    const onview = vi.fn();
    const c = mount({ ...views(), view: 'run', onview });
    const sw = c.querySelector('[data-lane-view]')!;
    const button = (label: string) => [...sw.querySelectorAll('button')].find((b) => text(b) === label)!;
    expect(button('By run')).toBeTruthy();
    button('By agent').click();
    expect(onview).toHaveBeenCalledWith('agent');
  });

  it('no callback, no switch', () => {
    expect(mount({ ...views(), view: 'run' }).querySelector('[data-lane-view]')).toBeNull();
  });
});
