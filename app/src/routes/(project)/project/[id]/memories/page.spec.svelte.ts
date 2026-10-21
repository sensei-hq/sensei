// @vitest-environment jsdom
import { describe, it, expect, afterEach, vi } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import Page from './+page.svelte';
import type { PageData } from './$types.js';
import { page } from '$app/state';

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

// #293: a write that failed is shown — a failed propose used to clear the
// selection and say nothing (the call fell back to a made-up `{ id: '' }`).
describe('Project Memories /+page — a failed propose', () => {
  it('keeps the selection and says why', async () => {
    page.params.id = 'p1';
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: false, status: 500, statusText: 'Internal Server Error' }));
    const memory = { id: 'm1', name: 'm1', title: 'a memory', kind: 'convention', type: 'convention', strength: 1, scope: 'project', generalised: false, generalisedContent: null };
    const m = mountComponent(Page, { data: { project: { id: 'p1', name: 'p' }, memories: [memory], total: 1, proposedBatches: [], error: null } as unknown as PageData });
    cleanup.push(m.destroy);
    (m.container.querySelector('[data-testid="memory-checkbox-m1"]') as HTMLInputElement).click();
    await vi.waitFor(() => expect(m.container.textContent).toContain('1 selected'));
    (m.container.querySelector('[data-testid="propose-batch-button"]') as HTMLButtonElement).click();
    await vi.waitFor(() =>
      expect(m.container.querySelector('[data-testid="propose-batch-error"]')?.textContent).toContain('Internal Server Error'),
    );
    expect(m.container.textContent).toContain('1 selected');
    vi.unstubAllGlobals();
  });
});
