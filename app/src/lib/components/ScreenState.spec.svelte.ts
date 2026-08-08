// @vitest-environment jsdom
import { describe, it, expect, afterEach, vi } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import Harness from './ScreenState.harness.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

describe('ScreenState', () => {
  it('renders children when ready', () => {
    const m = mountComponent(Harness, { status: 'ready' });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-testid="ready-content"]')).toBeTruthy();
  });

  it('shows a loading skeleton (not the children) when loading', () => {
    const m = mountComponent(Harness, { status: 'loading' });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-screen-state="loading"]')).toBeTruthy();
    expect(m.container.querySelector('[data-testid="ready-content"]')).toBeFalsy();
  });

  it('shows the empty state with kanji/title/description', () => {
    const m = mountComponent(Harness, {
      status: 'empty', kanji: '空', title: 'All clear', description: 'Nothing pending',
    });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-empty]')).toBeTruthy();
    expect(m.container.textContent).toContain('All clear');
    expect(m.container.querySelector('[data-testid="ready-content"]')).toBeFalsy();
  });

  // The core no-fabrication guarantee: an error is a DISTINCT state, never the
  // empty/ready content masquerading as success.
  it('shows error with its message and NOT the empty/ready content', () => {
    const m = mountComponent(Harness, { status: 'error', error: 'daemon unreachable' });
    cleanup.push(m.destroy);
    const err = m.container.querySelector('[data-screen-state="error"]');
    expect(err).toBeTruthy();
    expect(err?.getAttribute('role')).toBe('alert');
    expect(m.container.textContent).toContain('daemon unreachable');
    expect(m.container.querySelector('[data-empty]')).toBeFalsy();
    expect(m.container.querySelector('[data-testid="ready-content"]')).toBeFalsy();
  });

  it('falls back to a default error message when none is given', () => {
    const m = mountComponent(Harness, { status: 'error' });
    cleanup.push(m.destroy);
    const err = m.container.querySelector('[data-screen-state="error"]') as HTMLElement;
    expect((err.textContent ?? '').trim().length).toBeGreaterThan(0);
  });

  it('renders a Retry button that calls onretry', () => {
    const onretry = vi.fn();
    const m = mountComponent(Harness, { status: 'error', error: 'x', onretry });
    cleanup.push(m.destroy);
    const btn = m.container.querySelector('[data-action="retry"]') as HTMLButtonElement;
    expect(btn).toBeTruthy();
    btn.click();
    expect(onretry).toHaveBeenCalledOnce();
  });

  it('omits the Retry button when no onretry is provided', () => {
    const m = mountComponent(Harness, { status: 'error', error: 'x' });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-action="retry"]')).toBeFalsy();
  });
});
