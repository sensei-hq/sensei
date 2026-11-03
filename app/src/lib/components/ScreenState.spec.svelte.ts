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

  // The error branch is the shared Notice (docs/architecture/notice.md): the same
  // screen-size failure as the bootstrap, in sensei and the dōjō.
  describe('error renders through Notice', () => {
    const notice = (m: { container: HTMLElement }) =>
      m.container.querySelector('[data-component="notice"]') as HTMLElement;

    it('is a screen-size fault Notice', () => {
      const m = mountComponent(Harness, { status: 'error', error: 'daemon unreachable' });
      cleanup.push(m.destroy);
      const n = notice(m);
      expect(n.dataset.size).toBe('screen');
      expect(n.dataset.tone).toBe('fault');
      expect(n.dataset.screenState).toBe('error');
      expect(n.querySelector('[data-part="title"]')!.textContent).toContain("Couldn't load this");
      expect(n.querySelector('[data-part="detail"]')!.textContent).toBe('daemon unreachable');
    });

    it('Retry is the primary action', () => {
      const onretry = vi.fn();
      const m = mountComponent(Harness, { status: 'error', error: 'x', onretry });
      cleanup.push(m.destroy);
      const btn = notice(m).querySelector('[data-action="retry"]') as HTMLButtonElement;
      expect(btn.dataset.variant).toBe('primary');
    });

    it('a screen can say what failed and what to do, from its state module', () => {
      const m = mountComponent(Harness, {
        status: 'error',
        notice: { title: "Couldn't load tickets", detail: 'Failed to fetch', remedy: 'Check the daemon.', code: 'unreachable' },
      });
      cleanup.push(m.destroy);
      const n = notice(m);
      expect(n.querySelector('[data-part="title"]')!.textContent).toContain("Couldn't load tickets");
      expect(n.querySelector('[data-part="remedy"]')!.textContent).toContain('Check the daemon.');
      expect(n.querySelector('[data-part="code"]')!.textContent).toBe('unreachable');
    });

    it('with no message, says what to check rather than showing nothing', () => {
      const m = mountComponent(Harness, { status: 'error' });
      cleanup.push(m.destroy);
      expect(notice(m).querySelector('[data-part="remedy"]')!.textContent).toMatch(/daemon/);
    });
  });
});
