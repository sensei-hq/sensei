// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import Page from './+page.svelte';
import type { PageData } from './$types.js';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

// #288: a read that failed is shown as a failure — not an empty page.
describe('Project Memories /+page — a failed read', () => {
  it('shows the error with retry, not the empty page', () => {
    const m = mountComponent(Page, { data: { project: { id: 'p1', name: 'p' }, memories: [], total: 0, proposedBatches: [], error: 'Service Unavailable' } as unknown as PageData });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-screen-state="error"]')?.textContent).toContain('Service Unavailable');
    expect(m.container.textContent).not.toContain('Nothing to remember yet');
  });
});
