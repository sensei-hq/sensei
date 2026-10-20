// @vitest-environment jsdom
import { describe, it, expect, afterEach, vi } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import Page from './+page.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; vi.unstubAllGlobals(); });

// #288: memories that could not be read are an error with retry — not empty tabs.
describe('Learnings /+page — a failed read', () => {
  it('shows the error, and no tab list', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: false, status: 503, statusText: 'Service Unavailable', json: async () => ({}) }));
    const m = mountComponent(Page, {});
    cleanup.push(m.destroy);
    await vi.waitFor(() =>
      expect(m.container.querySelector('[data-screen-state="error"]')?.textContent).toContain('Service Unavailable'),
    );
  });
});
