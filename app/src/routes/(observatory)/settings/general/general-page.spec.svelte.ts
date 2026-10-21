// @vitest-environment jsdom
import { describe, it, expect, afterEach, vi } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import Page from './+page.svelte';
import { load } from './+page';

const loaded = async () => (await (load as unknown as () => Promise<unknown>)()) as never;
import { STORAGE_KEYS } from '$lib/storage-keys.js';

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
    destroy = mountComponent(Page, { data: await loaded() }).destroy;
    await new Promise((r) => setTimeout(r, 200));
    // Nothing changed, so nothing is saved — the read is not written back.
    expect(puts.length).toBe(0);
  });

  it('saves the system user name as the display name when none is set', async () => {
    vi.stubGlobal('localStorage', {
      getItem: (k: string) => (k === STORAGE_KEYS.userName ? 'Jerry' : null),
      setItem: () => {},
      removeItem: () => {},
    });
    const puts: string[] = [];
    vi.stubGlobal(
      'fetch',
      vi.fn(async (url: string, init?: RequestInit) => {
        if (init?.method === 'PUT') puts.push(String(init.body));
        return new Response(JSON.stringify({}), { status: 200 });
      }),
    );
    const m = mountComponent(Page, { data: await loaded() });
    destroy = m.destroy;
    await new Promise((r) => setTimeout(r, 200));
    const field = m.container.querySelector('[data-testid="pref-display-name"]') as HTMLInputElement;
    expect(field.value).toBe('Jerry');
    expect(puts.some((b) => b.includes('"user_name"') && b.includes('Jerry'))).toBe(true);
  });
});
