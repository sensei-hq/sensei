// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import SprintBand from './SprintBand.svelte';
import * as F from '$lib/test-stubs/relay-fixtures.js';

// Queue cutover (phase 4): one band per open run above the lanes — title,
// state, goal, the counts by state, and done over total as the progress.

let cleanup: Array<() => void> = [];
afterEach(() => {
  cleanup.forEach((fn) => fn());
  cleanup = [];
});

function mount(props: { run: ReturnType<typeof F.sprint> }) {
  const m = mountComponent(SprintBand, props);
  cleanup.push(m.destroy);
  const root = m.container.querySelector('[data-sprint-band]') as HTMLElement;
  return { ...m, root };
}

const count = (root: HTMLElement, key: string) =>
  (root.querySelector(`[data-count="${key}"]`) as HTMLElement | null)?.textContent?.trim();

describe('SprintBand', () => {
  it('names the run and counts its tasks by state', () => {
    const { root } = mount({ run: F.sprint() });
    expect(root.dataset.runId).toBe(F.RUN_ID);
    expect(root.textContent).toContain('Sprint 14: Broker hardening');
    expect(root.textContent).toContain('Resolve the high-severity broker findings');
    expect(count(root, 'ready')).toBe('2 ready');
    expect(count(root, 'working')).toBe('1 working');
    expect(count(root, 'in_review')).toBe('1 in review · 1 floating');
    expect(count(root, 'done')).toBe('2 done');
    expect(root.querySelector('[data-count="approved"]')).toBeNull();
    expect(root.querySelector('[data-count="blocked"]')).toBeNull();
  });

  it('progress is done over total', () => {
    const { root } = mount({ run: F.sprint() });
    const progress = root.querySelector('[data-progress]') as HTMLElement;
    expect(progress.dataset.done).toBe('2');
    expect(progress.dataset.total).toBe('6');
    expect(progress.dataset.fraction).toBe('0.33');
    expect(progress.textContent).toContain('2 of 6');
  });

  it('a planning run reads as planning with Task 0 in the queue', () => {
    const { root } = mount({
      run: F.sprint({ state: 'planning', total_tasks: 1, ready_tasks: 1, working_tasks: 0, in_review_tasks: 0, floating_reviews: 0, done_tasks: 0, started_at: null }),
    });
    const state = root.querySelector('[data-state]') as HTMLElement;
    expect(state.textContent?.trim()).toBe('planning · Task 0 in the queue');
    expect((root.querySelector('[data-progress]') as HTMLElement).dataset.fraction).toBe('0');
  });

  it('zero tasks reads planning, never a division', () => {
    const { root } = mount({
      run: F.sprint({ state: 'planning', total_tasks: 0, ready_tasks: 0, working_tasks: 0, in_review_tasks: 0, floating_reviews: 0, done_tasks: 0, started_at: null }),
    });
    expect((root.querySelector('[data-state]') as HTMLElement).textContent?.trim()).toBe('planning');
    expect((root.querySelector('[data-progress]') as HTMLElement).dataset.fraction).toBe('0');
  });

  it('a blocked count shows when it is not zero', () => {
    const { root } = mount({ run: F.sprint({ blocked_tasks: 1, approved_tasks: 2 }) });
    expect(count(root, 'blocked')).toBe('1 blocked');
    expect(count(root, 'approved')).toBe('2 approved');
  });
});
