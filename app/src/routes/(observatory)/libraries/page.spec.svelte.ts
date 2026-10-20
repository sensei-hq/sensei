// @vitest-environment jsdom
import { describe, it, expect, afterEach, vi } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import Page from './+page.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; vi.unstubAllGlobals(); });

// #288: a library list that could not be read is an error with retry — not
// "No libraries indexed."
describe('Libraries /+page — a failed read', () => {
  it('shows the error, not the empty state', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: false, status: 503, statusText: 'Service Unavailable' }));
    const m = mountComponent(Page, {});
    cleanup.push(m.destroy);
    await vi.waitFor(() =>
      expect(m.container.querySelector('[data-screen-state="error"]')?.textContent).toContain('Service Unavailable'),
    );
    expect(m.container.textContent).not.toContain('No libraries indexed.');
  });
});
