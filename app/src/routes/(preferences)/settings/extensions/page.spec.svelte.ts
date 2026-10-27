// @vitest-environment jsdom
import { describe, it, expect, afterEach, vi } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import Page from './+page.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; vi.unstubAllGlobals(); });

// #293: a refused toggle was a console warning and a silent re-read.
describe('Settings · Extensions — a refused toggle', () => {
  it('says why on the row and puts the box back', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: false, status: 409, statusText: 'Conflict' }));
    const data = { extensions: [{ name: 'tdd', kind: 'skill', enabled: true }], error: null };
    const m = mountComponent(Page, { data } as never);
    cleanup.push(m.destroy);
    const box = m.container.querySelector('[data-testid="ext-toggle-skill-tdd"]') as HTMLInputElement;
    box.click();
    await vi.waitFor(() =>
      expect(m.container.querySelector('[data-testid="ext-error-skill-tdd"]')?.textContent).toContain('Conflict'),
    );
    expect(box.checked).toBe(true);
  });
});
