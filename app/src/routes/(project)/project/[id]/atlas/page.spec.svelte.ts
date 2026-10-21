// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import Page from './+page.svelte';
import type { PageData } from './$types.js';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

// #288: an atlas that could not be read is an error with retry — not an empty
// graph with zero modules, exports and calls.
describe('Atlas /+page — a failed read', () => {
  it('shows the error, naming the read, and draws no atlas', () => {
    const error = 'Could not read the atlas — call flow: Service Unavailable';
    const m = mountComponent(Page, { data: { repoId: 'repo-a', scopes: [], atlas: null, error } as unknown as PageData });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-screen-state="error"]')?.textContent).toContain('call flow: Service Unavailable');
    expect(m.container.querySelector('[data-screen="atlas"]')).toBeNull();
  });
});
