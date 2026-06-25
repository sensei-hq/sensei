// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import ProgressCardHarness from './ProgressCard.harness.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

describe('ProgressCard', () => {
  it('renders label, progressbar value, and optional rows', () => {
    const m = mountComponent(ProgressCardHarness, {
      label: 'Installing', trailing: '≈ 2 min left', percent: 58,
      left: '3 of 6 ready', right: '1.4 / 2.1 GB',
      activity: '==> Pouring ollama', note: 'Runs once.',
    });
    cleanup.push(m.destroy);
    const card = m.container.querySelector('[data-component="progress-card"]') as HTMLElement;
    expect(card).not.toBeNull();
    expect(card.textContent).toContain('Installing');
    expect(card.textContent).toContain('≈ 2 min left');
    expect(card.textContent).toContain('3 of 6 ready');
    expect(card.textContent).toContain('1.4 / 2.1 GB');
    expect(card.textContent).toContain('==> Pouring ollama');
    expect(card.textContent).toContain('Runs once.');
    const bar = card.querySelector('[role="progressbar"]') as HTMLElement;
    expect(bar.getAttribute('aria-valuenow')).toBe('58');
    expect(bar.querySelector('div')?.getAttribute('style')).toContain('width: 58%');
  });

  it('clamps percent to 0–100', () => {
    const m = mountComponent(ProgressCardHarness, { label: 'x', percent: 250 });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[role="progressbar"]')?.getAttribute('aria-valuenow')).toBe('100');
  });

  it('omits optional rows when not provided', () => {
    const m = mountComponent(ProgressCardHarness, { label: 'Scanning', percent: 10 });
    cleanup.push(m.destroy);
    const card = m.container.querySelector('[data-component="progress-card"]') as HTMLElement;
    expect(card.querySelector('[data-component="spinner"]')).toBeNull(); // no activity → no spinner
    expect(card.textContent).toContain('Scanning');
  });
});
