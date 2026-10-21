// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import Page from './+page.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

// #288: a list that could not be read is an error with retry — not the
// "no projects" an empty fallback list rendered.
describe('Projects /+page — a failed list read', () => {
  it('shows the error, and no project counts', () => {
    const m = mountComponent(Page, { data: { buckets: null, error: 'Service Unavailable' } });
    cleanup.push(m.destroy);
    const state = m.container.querySelector('[data-screen-state="error"]');
    expect(state?.textContent).toContain('Service Unavailable');
    expect(m.container.querySelector('[data-project-card], [data-project-row]')).toBeNull();
  });
});
