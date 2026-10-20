// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import Page from './+page.svelte';
import type { PageData } from './$types.js';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

// #288: a board that could not be read is an error with retry — not the
// three quiet columns an empty fallback board rendered.
describe('Insights /+page — a failed read', () => {
  it('shows the error in place of the board', () => {
    const m = mountComponent(Page, { data: { board: null, error: 'Service Unavailable' } as unknown as PageData });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-screen-state="error"]')?.textContent).toContain('Service Unavailable');
    expect(m.container.querySelector('[data-triage-grid]')).toBeNull();
  });
});
