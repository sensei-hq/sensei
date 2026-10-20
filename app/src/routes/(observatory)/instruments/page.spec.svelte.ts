// @vitest-environment jsdom
import { describe, it, expect, afterEach, vi } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import Page from './+page.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; vi.unstubAllGlobals(); });

// #288: a tool catalog that could not be read is said so — sensei's own tools
// were otherwise just missing from the Playground rail.
describe('Instruments /+page — a failed catalog read', () => {
  it('shows the catalog error in the Playground', async () => {
    const fetch = vi.fn().mockResolvedValue({ ok: false, status: 503, statusText: 'Service Unavailable', json: async () => ({}) });
    vi.stubGlobal('fetch', fetch);
    const m = mountComponent(Page, {});
    cleanup.push(m.destroy);
    await vi.waitFor(() =>
      expect(m.container.querySelector('[data-screen-state="error"]')?.textContent).toContain('Could not read the tool catalog: Service Unavailable'),
    );
    // And it stops asking: the tab effect used to re-run on every status
    // change, so a failing read was re-fetched without end.
    await new Promise((r) => setTimeout(r, 200));
    const settled = fetch.mock.calls.length;
    await new Promise((r) => setTimeout(r, 300));
    expect(fetch.mock.calls.length).toBe(settled);
  });
});
