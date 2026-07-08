// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import MemoryRowHarness from './MemoryRow.harness.svelte';
import type { MemoryRowVm } from './insights-board.svelte.js';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

const memory = (over: Partial<MemoryRowVm> = {}): MemoryRowVm => ({
  id: 'm1',
  title: 'Use dbd deploy, not combine',
  content: 'combine mangles long comments',
  strength: 1,
  scope: 'project',
  violatedCount: 0,
  ...over,
});

describe('MemoryRow', () => {
  it('settled variant shows the strength bar and scope', () => {
    const m = mountComponent(MemoryRowHarness, { memory: memory({ strength: 0.6 }), variant: 'settled' });
    cleanup.push(m.destroy);
    const el = m.container.querySelector('[data-component="memory-row"]');
    expect(el?.getAttribute('data-variant')).toBe('settled');
    const fill = m.container.querySelector('[data-strength-fill]') as HTMLElement;
    expect(fill).not.toBeNull();
    expect(fill.getAttribute('style')).toContain('60%');
    expect(m.container.textContent).toContain('project');
    expect(m.container.textContent).toContain('Use dbd deploy, not combine');
  });

  it('proposed variant shows the proposed label and no strength bar', () => {
    const m = mountComponent(MemoryRowHarness, { memory: memory(), variant: 'proposed' });
    cleanup.push(m.destroy);
    const el = m.container.querySelector('[data-component="memory-row"]');
    expect(el?.getAttribute('data-variant')).toBe('proposed');
    expect(m.container.textContent).toContain('proposed');
    expect(m.container.querySelector('[data-strength-fill]')).toBeNull();
  });

  it('is display-only — no write buttons', () => {
    const m = mountComponent(MemoryRowHarness, { memory: memory(), variant: 'settled' });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('button')).toBeNull();
  });
});
