// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import StatusDiscHarness from './StatusDisc.harness.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

describe('StatusDisc', () => {
  it('renders the disc element', () => {
    const m = mountComponent(StatusDiscHarness, { status: 'pending' });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-component="status-disc"]')).toBeTruthy();
  });

  it('renders check glyph when status="ready"', () => {
    const m = mountComponent(StatusDiscHarness, { status: 'ready' });
    cleanup.push(m.destroy);
    const el = m.container.querySelector('[data-component="status-disc"]') as HTMLElement;
    expect(el.className).toMatch(/\bborder-success\b/);
    expect(el.querySelector('svg')).toBeTruthy();
  });

  it('renders spinner when status="checking"', () => {
    const m = mountComponent(StatusDiscHarness, { status: 'checking' });
    cleanup.push(m.destroy);
    const el = m.container.querySelector('[data-component="status-disc"]') as HTMLElement;
    expect(el.className).toMatch(/\bborder-accent\b/);
    expect(el.querySelector('[data-component="spinner"]')).toBeTruthy();
  });

  it('renders spinner when status="installing"', () => {
    const m = mountComponent(StatusDiscHarness, { status: 'installing' });
    cleanup.push(m.destroy);
    const el = m.container.querySelector('[data-component="status-disc"]') as HTMLElement;
    expect(el.querySelector('[data-component="spinner"]')).toBeTruthy();
  });

  it('renders question kanji when status="failed"', () => {
    const m = mountComponent(StatusDiscHarness, { status: 'failed' });
    cleanup.push(m.destroy);
    const el = m.container.querySelector('[data-component="status-disc"]') as HTMLElement;
    expect(el.className).toMatch(/\bborder-accent\b/);
    expect(el.textContent).toContain('?');
  });

  it('uses muted border when status="pending"', () => {
    const m = mountComponent(StatusDiscHarness, { status: 'pending' });
    cleanup.push(m.destroy);
    const el = m.container.querySelector('[data-component="status-disc"]') as HTMLElement;
    expect(el.className).toMatch(/\bborder-ink-faint\b/);
  });

  it('applies provided size (default 20)', () => {
    const m = mountComponent(StatusDiscHarness, { status: 'pending', size: 32 });
    cleanup.push(m.destroy);
    const el = m.container.querySelector('[data-component="status-disc"]') as HTMLElement;
    expect(el.style.width).toBe('32px');
    expect(el.style.height).toBe('32px');
  });
});
