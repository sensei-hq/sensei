// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import Page from './+page.svelte';
import type { PageData } from './$types.js';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

// #288: a read that failed is shown as a failure — not an empty screen.
describe('Project impact /+page — a failed read', () => {
  it('shows the error with retry, and no counts', () => {
    const m = mountComponent(Page, { data: { project: { id: 'p1', name: 'p' }, buckets: { positive: [], neutral: [], negative: [], pending: [] }, total: 0, impactLog: [], error: 'Service Unavailable' } as unknown as PageData });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-screen-state="error"]')?.textContent).toContain('Service Unavailable');
    expect(m.container.textContent).not.toMatch(/\b0 tracked|no measurements yet|no drift detected/);
  });
});
