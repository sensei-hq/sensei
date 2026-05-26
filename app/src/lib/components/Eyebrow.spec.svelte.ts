// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import EyebrowHarness from './Eyebrow.harness.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

describe('Eyebrow', () => {
  it('renders text content', () => {
    const m = mountComponent(EyebrowHarness, { label: 'Sessions' });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toContain('Sessions');
  });

  it('uppercases via class (does not transform input string)', () => {
    const m = mountComponent(EyebrowHarness, { label: 'first-try-right · 14d' });
    cleanup.push(m.destroy);
    const el = m.container.querySelector('[data-component="eyebrow"]') as HTMLElement;
    expect(el).toBeTruthy();
    expect(el.textContent).toContain('first-try-right · 14d');
    expect(el.className).toMatch(/\buppercase\b/);
  });

  it('applies the wide tracking + xs size by default', () => {
    const m = mountComponent(EyebrowHarness, { label: 'X' });
    cleanup.push(m.destroy);
    const el = m.container.querySelector('[data-component="eyebrow"]') as HTMLElement;
    expect(el.className).toMatch(/\btracking-wide\b/);
    expect(el.className).toMatch(/\btext-xs\b/);
  });

  it('uses muted tone (ink-z6) by default', () => {
    const m = mountComponent(EyebrowHarness, { label: 'X' });
    cleanup.push(m.destroy);
    const el = m.container.querySelector('[data-component="eyebrow"]') as HTMLElement;
    expect(el.className).toMatch(/\btext-ink-z6\b/);
  });

  it('uses ink tone (ink-z9) when tone="ink"', () => {
    const m = mountComponent(EyebrowHarness, { label: 'X', tone: 'ink' });
    cleanup.push(m.destroy);
    const el = m.container.querySelector('[data-component="eyebrow"]') as HTMLElement;
    expect(el.className).toMatch(/\btext-ink-z9\b/);
  });
});
