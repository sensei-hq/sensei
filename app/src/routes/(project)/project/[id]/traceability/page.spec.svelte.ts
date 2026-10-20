// @vitest-environment jsdom
import { describe, it, expect, afterEach, vi } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import Page from './+page.svelte';
import type { PageData } from './$types.js';
import { page } from '$app/state';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

// #288: a read that failed is shown as a failure — not an empty screen.
describe('Project traceability /+page — a failed read', () => {
  it('shows the error with retry, and no counts', () => {
    const m = mountComponent(Page, { data: { project: { id: 'p1', name: 'p' }, driftItems: [], total: 0, drifted: 0, broken: 0, error: 'Service Unavailable' } as unknown as PageData });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-screen-state="error"]')?.textContent).toContain('Service Unavailable');
    expect(m.container.textContent).not.toMatch(/\b0 tracked|no measurements yet|no drift detected/);
  });

  it('says when a scan failed, rather than doing nothing', async () => {
    page.params.id = 'p1';
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: false, status: 500, statusText: 'Internal Server Error' }));
    const m = mountComponent(Page, {
      data: { project: { id: 'p1', name: 'p' }, driftItems: [], total: 3, drifted: 0, broken: 0, error: null } as unknown as PageData,
    });
    cleanup.push(m.destroy);
    (m.container.querySelector('[data-testid="drift-scan-button"]') as HTMLButtonElement).click();
    await vi.waitFor(() =>
      expect(m.container.querySelector('[data-testid="drift-scan-error"]')?.textContent).toContain('Internal Server Error'),
    );
    vi.unstubAllGlobals();
  });
});
