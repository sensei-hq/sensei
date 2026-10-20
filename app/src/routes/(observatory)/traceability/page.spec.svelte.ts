// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import Page from './+page.svelte';
import type { PageData } from './$types.js';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

// #288: a read that failed is shown as a failure — not an empty screen.
describe('Traceability /+page — a failed read', () => {
  it('shows the error with retry, and no counts', () => {
    const m = mountComponent(Page, { data: { groups: null, rollup: null, unread: [], error: 'Service Unavailable' } as unknown as PageData });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-screen-state="error"]')?.textContent).toContain('Service Unavailable');
    expect(m.container.textContent).not.toMatch(/\b0 tracked|no measurements yet|no drift detected/);
  });

  it('names the projects it could not read beside what it did', () => {
    const m = mountComponent(Page, { data: { groups: [], rollup: { total: 0, drifted: 0, broken: 0, projectsAffected: 0 }, unread: [{ name: 'beta', reason: 'Bad Gateway' }], error: null } as unknown as PageData });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-unread-projects]')?.textContent).toContain('beta (Bad Gateway)');
  });
});
