// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import ViolationCardHarness from './ViolationCard.harness.svelte';
import type { ViolationCardVm } from './insights-board.svelte.js';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

const memory = (over: Partial<ViolationCardVm> = {}): ViolationCardVm => ({
  id: 'm1',
  projectId: 'p1',
  title: 'Never build against the live dev server',
  content: 'A reload storm caused a partial render on 2026-05-11',
  violatedCount: 3,
  ...over,
});

describe('ViolationCard', () => {
  it('renders the violated count and the memory title', () => {
    const m = mountComponent(ViolationCardHarness, { memory: memory() });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-violation-count]')?.textContent).toContain('3');
    expect(m.container.textContent).toContain('Never build against the live dev server');
  });

  it('is display-only — no write buttons (memory actions are deferred)', () => {
    const m = mountComponent(ViolationCardHarness, { memory: memory() });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('button')).toBeNull();
  });

  it('omits the body line when content is empty', () => {
    const m = mountComponent(ViolationCardHarness, { memory: memory({ content: '' }) });
    cleanup.push(m.destroy);
    expect(m.container.textContent).not.toContain('reload storm');
  });
});
