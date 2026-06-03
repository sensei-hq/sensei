// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import StatusIndicatorHarness from './StatusIndicator.harness.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

describe('StatusIndicator', () => {
  it('renders no label when status="pending" and no label provided', () => {
    const m = mountComponent(StatusIndicatorHarness, { status: 'pending' });
    cleanup.push(m.destroy);
    const label = m.container.querySelector('[data-component="status-indicator-label"]');
    expect(label).toBeNull();
  });

  it('renders the disc for every status', () => {
    const m = mountComponent(StatusIndicatorHarness, { status: 'pending' });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-component="status-disc"]')).toBeTruthy();
  });

  it('renders "checking" label for status="checking"', () => {
    const m = mountComponent(StatusIndicatorHarness, { status: 'checking' });
    cleanup.push(m.destroy);
    const label = m.container.querySelector('[data-component="status-indicator-label"]') as HTMLElement;
    expect(label.textContent?.trim()).toBe('checking');
    expect(label.className).toMatch(/\btext-accent\b/);
    expect(label.className).toMatch(/\bfont-mono\b/);
    expect(label.className).toMatch(/\buppercase\b/);
  });

  it('renders no label when status="ready" (disc alone communicates ready)', () => {
    const m = mountComponent(StatusIndicatorHarness, { status: 'ready' });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-component="status-indicator-label"]')).toBeNull();
  });

  it('renders "blocked" label for status="failed"', () => {
    const m = mountComponent(StatusIndicatorHarness, { status: 'failed' });
    cleanup.push(m.destroy);
    const label = m.container.querySelector('[data-component="status-indicator-label"]') as HTMLElement;
    expect(label.textContent?.trim()).toBe('blocked');
    expect(label.className).toMatch(/\btext-accent\b/);
  });

  it('uses provided label for installing (installingVerb override)', () => {
    const m = mountComponent(StatusIndicatorHarness, { status: 'installing', label: 'starting' });
    cleanup.push(m.destroy);
    const label = m.container.querySelector('[data-component="status-indicator-label"]') as HTMLElement;
    expect(label.textContent?.trim()).toBe('starting');
  });
});
