// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import SpinnerHarness from './Spinner.harness.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

describe('Spinner', () => {
  it('renders the spinner element', () => {
    const m = mountComponent(SpinnerHarness, { size: 10, tone: 'accent' });
    cleanup.push(m.destroy);
    const el = m.container.querySelector('[data-component="spinner"]') as HTMLElement;
    expect(el).toBeTruthy();
  });

  it('applies tone="accent" class', () => {
    const m = mountComponent(SpinnerHarness, { size: 10, tone: 'accent' });
    cleanup.push(m.destroy);
    const el = m.container.querySelector('[data-component="spinner"]') as HTMLElement;
    expect(el.className).toMatch(/\bborder-accent\b/);
  });

  it('applies tone="success" class', () => {
    const m = mountComponent(SpinnerHarness, { size: 14, tone: 'success' });
    cleanup.push(m.destroy);
    const el = m.container.querySelector('[data-component="spinner"]') as HTMLElement;
    expect(el.className).toMatch(/\bborder-success\b/);
  });

  it('applies tone="ink" class', () => {
    const m = mountComponent(SpinnerHarness, { size: 10, tone: 'ink' });
    cleanup.push(m.destroy);
    const el = m.container.querySelector('[data-component="spinner"]') as HTMLElement;
    expect(el.className).toMatch(/\bborder-ink\b/);
  });

  it('uses provided size', () => {
    const m = mountComponent(SpinnerHarness, { size: 12, tone: 'accent' });
    cleanup.push(m.destroy);
    const el = m.container.querySelector('[data-component="spinner"]') as HTMLElement;
    expect(el.style.width).toBe('12px');
    expect(el.style.height).toBe('12px');
  });
});
