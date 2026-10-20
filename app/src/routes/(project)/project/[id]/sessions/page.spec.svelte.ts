// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import Page from './+page.svelte';
import type { PageData } from './$types.js';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

// #288: sessions that could not be read are an error with retry — not an
// empty digest that reads as a quiet week. The range chips stay usable.
describe('Sessions /+page — a failed read', () => {
  it('shows the error in place of the digest, and no zero totals', () => {
    const m = mountComponent(Page, { data: { project: { id: 'p1', name: 'sensei' }, projectId: 'p1', sessions: null, range: '7d', error: 'Service Unavailable' } as unknown as PageData });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-screen-state="error"]')?.textContent).toContain('Service Unavailable');
    expect(m.container.querySelector('[data-testid="session-list"], [data-testid="chart-body"], [data-testid="mini-numbers"]')).toBeNull();
    expect(m.container.querySelector('[data-testid="range-chips"]')).not.toBeNull();
  });
});
