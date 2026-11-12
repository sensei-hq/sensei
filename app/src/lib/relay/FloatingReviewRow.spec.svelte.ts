// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import FloatingReviewRow from './FloatingReviewRow.svelte';
import * as F from '$lib/test-stubs/relay-fixtures.js';

// Queue cutover (phase 4): a report waiting for a reviewer — the task, who
// wrote it, which turn, and how long it has floated.

let cleanup: Array<() => void> = [];
afterEach(() => {
  cleanup.forEach((fn) => fn());
  cleanup = [];
});

function mount(props: { review: ReturnType<typeof F.floatingReview>; href: string }) {
  const m = mountComponent(FloatingReviewRow, props);
  cleanup.push(m.destroy);
  const root = m.container.querySelector('[data-floating-review]') as HTMLElement;
  return { ...m, root };
}

describe('FloatingReviewRow', () => {
  it('links the task and names the author by code and title', () => {
    const { root } = mount({ review: F.floatingReview(), href: '/relay/c/t-claim-handlers' });
    expect(root.dataset.taskId).toBe('t-claim-handlers');
    const link = root.querySelector('a[data-review-title]') as HTMLAnchorElement;
    expect(link.textContent?.trim()).toBe('Add claim handlers');
    expect(link.getAttribute('href')).toBe('/relay/c/t-claim-handlers');
    expect((root.querySelector('[data-author]') as HTMLElement).textContent?.trim()).toBe('L1 · Diagrams');
    expect((root.querySelector('[data-turn]') as HTMLElement).textContent?.trim()).toBe('impl · r1');
    expect((root.querySelector('[data-since]') as HTMLElement).textContent?.trim()).not.toBe('');
  });

  it('an untitled author reads by code alone', () => {
    const { root } = mount({ review: F.floatingReview({ author_lane_title: null, author_lane_code: 'L3' }), href: '/x' });
    expect((root.querySelector('[data-author]') as HTMLElement).textContent?.trim()).toBe('L3');
  });
});
