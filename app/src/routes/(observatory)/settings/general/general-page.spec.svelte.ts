// @vitest-environment jsdom
import { describe, it, expect, afterEach, vi } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import Page from './+page.svelte';

// The page auto-saves its toggles from an $effect. Saving goes through
// appState.setConfigs (#133), which reads appState.config — so the effect must
// not track that read, or every save re-runs the effect and saves again: an
// endless write loop that starved the page in the real app.
describe('Settings → General', () => {
  let destroy: (() => void) | null = null;
  afterEach(() => {
    destroy?.();
    destroy = null;
    vi.unstubAllGlobals();
  });

  it('does not save in a loop once loaded', async () => {
    const puts: string[] = [];
    vi.stubGlobal(
      'fetch',
      vi.fn(async (url: string, init?: RequestInit) => {
        if (init?.method === 'PUT') puts.push(String(init.body));
        return new Response(JSON.stringify({}), { status: 200 });
      }),
    );
    destroy = mountComponent(Page, {}).destroy;
    await new Promise((r) => setTimeout(r, 200));
    expect(puts.length).toBeLessThanOrEqual(1);
  });
});
