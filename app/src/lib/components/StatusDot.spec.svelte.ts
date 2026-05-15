// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import StatusDot from './StatusDot.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

const root = (m: { container: HTMLElement }): HTMLElement =>
  m.container.querySelector('[data-component="status-dot"]') as HTMLElement;

describe('StatusDot', () => {
  it('renders a non-empty element', () => {
    const m = mountComponent(StatusDot, { status: 'ok' });
    cleanup.push(m.destroy);
    expect(root(m)).toBeTruthy();
  });

  it('is a round inline element by default', () => {
    const m = mountComponent(StatusDot, { status: 'ok' });
    cleanup.push(m.destroy);
    expect(root(m).className).toMatch(/\binline-block\b/);
    expect(root(m).className).toMatch(/\brounded-full\b/);
  });

  it.each([
    ['ok',   /\bbg-success-z6\b/],
    ['busy', /\bbg-primary-z6\b/],
    ['warn', /\bbg-warning-z6\b/],
    ['fail', /\bbg-primary-z6\b/],
    ['idle', /\bbg-ink-z5\b/],
  ] as const)('status %s maps to %s', (status, expected) => {
    const m = mountComponent(StatusDot, { status });
    cleanup.push(m.destroy);
    expect(root(m).className).toMatch(expected);
  });

  it.each([
    ['sm', /\bw-1\.5\b/, /\bh-1\.5\b/],
    ['md', /\bw-2\b/,    /\bh-2\b/],
    ['lg', /\bw-2\.5\b/, /\bh-2\.5\b/],
  ] as const)('size %s applies width + height utilities', (size, wRe, hRe) => {
    const m = mountComponent(StatusDot, { status: 'ok', size });
    cleanup.push(m.destroy);
    expect(root(m).className).toMatch(wRe);
    expect(root(m).className).toMatch(hRe);
  });

  it('defaults to size md', () => {
    const m = mountComponent(StatusDot, { status: 'ok' });
    cleanup.push(m.destroy);
    expect(root(m).className).toMatch(/\bw-2\b/);
  });

  it('exposes an aria-label when label prop is set', () => {
    const m = mountComponent(StatusDot, { status: 'warn', label: 'drifted' });
    cleanup.push(m.destroy);
    expect(root(m).getAttribute('aria-label')).toBe('drifted');
    expect(root(m).getAttribute('role')).toBe('status');
  });

  it('has no aria-label / role when label is absent', () => {
    const m = mountComponent(StatusDot, { status: 'ok' });
    cleanup.push(m.destroy);
    expect(root(m).hasAttribute('aria-label')).toBe(false);
    expect(root(m).getAttribute('aria-hidden')).toBe('true');
  });
});
