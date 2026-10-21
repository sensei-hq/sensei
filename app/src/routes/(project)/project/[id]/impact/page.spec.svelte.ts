// @vitest-environment jsdom
import { describe, it, expect, afterEach, vi } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import Page from './+page.svelte';
import type { PageData } from './$types.js';
import { page } from '$app/state';

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

// #304: a failed log keeps what was typed and says why — it was a console
// warning, and the form cleared as if the impact had been logged.
describe('Project impact /+page — a failed log', () => {
  it('keeps the title and shows the reason', async () => {
    page.params.id = 'p1';
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: false, status: 500, statusText: 'Internal Server Error' }));
    const m = mountComponent(Page, { data: { project: { id: 'p1', name: 'p' }, buckets: { positive: [], neutral: [], negative: [], pending: [] }, total: 0, impactLog: [], error: null } as unknown as PageData });
    cleanup.push(m.destroy);
    const title = m.container.querySelector('input') as HTMLInputElement;
    title.value = 'faster reviews';
    title.dispatchEvent(new Event('input', { bubbles: true }));
    await vi.waitFor(() => expect((m.container.querySelector('[data-testid="impact-log-button"]') as HTMLButtonElement).disabled).toBe(false));
    (m.container.querySelector('[data-testid="impact-log-button"]') as HTMLButtonElement).click();
    await vi.waitFor(() =>
      expect(m.container.querySelector('[data-testid="impact-log-error"]')?.textContent).toContain('Internal Server Error'),
    );
    expect(title.value).toBe('faster reviews');
    vi.unstubAllGlobals();
  });
});
