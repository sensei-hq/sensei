// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import DaemonStatusBanner from './DaemonStatusBanner.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

const root = (m: { container: HTMLElement }): HTMLElement | null =>
  m.container.querySelector('[data-component="daemon-status-banner"]');

describe('DaemonStatusBanner', () => {
  it('renders a status banner when the daemon is degraded', () => {
    const m = mountComponent(DaemonStatusBanner, { mode: 'degraded' });
    cleanup.push(m.destroy);
    const el = root(m);
    expect(el).toBeTruthy();
    expect(el!.getAttribute('role')).toBe('status');
    expect(el!.textContent).toMatch(/reconnecting/i);
  });

  it('renders nothing when the daemon is full', () => {
    const m = mountComponent(DaemonStatusBanner, { mode: 'full' });
    cleanup.push(m.destroy);
    expect(root(m)).toBeNull();
  });

  it('renders nothing when the mode is unknown', () => {
    const m = mountComponent(DaemonStatusBanner, {});
    cleanup.push(m.destroy);
    expect(root(m)).toBeNull();
  });

  // A failed settings read keeps the last settings (appState); the window says so
  // rather than quietly running on something it could not read.
  it('says when the settings could not be read, and that the last ones are kept', () => {
    const m = mountComponent(DaemonStatusBanner, { mode: 'full', configError: 'Load failed' });
    cleanup.push(m.destroy);
    const el = root(m);
    expect(el).toBeTruthy();
    expect(el!.getAttribute('role')).toBe('status');
    expect(el!.textContent).toMatch(/settings/i);
    expect(el!.textContent).toMatch(/Load failed/);
  });
});
