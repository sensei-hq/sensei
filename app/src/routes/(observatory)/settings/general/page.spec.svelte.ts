// @vitest-environment jsdom
import { describe, it, expect, afterEach, vi } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import Page from './+page.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; vi.unstubAllGlobals(); });

// #288: a config read that failed must not become a form of defaults — the
// page auto-saves, so that form would be written over the user's settings.
describe('Settings · General — a failed read', () => {
  it('shows the error and writes nothing', async () => {
    const fetch = vi.fn().mockResolvedValue({ ok: false, status: 503, statusText: 'Service Unavailable', json: async () => ({}) });
    vi.stubGlobal('fetch', fetch);
    const m = mountComponent(Page, {});
    cleanup.push(m.destroy);
    await vi.waitFor(() =>
      expect(m.container.querySelector('[data-screen-state="error"]')?.textContent).toContain('Service Unavailable'),
    );
    await new Promise((r) => setTimeout(r, 50));
    const writes = fetch.mock.calls.filter(([, init]) => init && (init as RequestInit).method && (init as RequestInit).method !== 'GET');
    expect(writes).toEqual([]);
    expect(m.container.querySelector('[data-testid="settings-general"]')).toBeNull();
  });
});
