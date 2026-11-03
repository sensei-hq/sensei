// @vitest-environment jsdom
import { describe, it, expect, afterEach, vi } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import type { KnowledgeSource } from '$lib/types.js';
import Page from './+page.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; vi.unstubAllGlobals(); });

const source = (id: string, over: Partial<KnowledgeSource> = {}): KnowledgeSource => ({
  id,
  kind: 'hive',
  name: `Hive ${id}`,
  url: `https://${id}.example.com`,
  namespace_id: null,
  direction: 'pull',
  last_seq: 12,
  enabled: true,
  ...over,
});

const sources = [source('s1'), source('s2')];

const okSources = (list: KnowledgeSource[]) => ({ ok: true, status: 200, json: async () => ({ sources: list }) });

// #288: a source list that could not be read is an error — never a silent
// list. The loader's read is where the list comes from once the fetch moves
// to +page.ts; the fixture carries both answers so the spec passes before and
// after with the same assertions.
describe('Knowledge sources /+page — a failed read', () => {
  it('shows the error, not a silent list', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: false, status: 503, statusText: 'Service Unavailable' }));
    const m = mountComponent(Page, { data: { sources: [], error: 'Service Unavailable' } } as never);
    cleanup.push(m.destroy);
    await vi.waitFor(() =>
      expect(m.container.textContent).toContain('Service Unavailable'),
    );
  });
});

// The honest empty: no sources, no error. Never the error state.
describe('Knowledge sources /+page — no sources', () => {
  it('shows the empty state, not the error state', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(okSources([])));
    const m = mountComponent(Page, { data: { sources: [], error: null } } as never);
    cleanup.push(m.destroy);
    await vi.waitFor(() => expect(m.container.textContent).toContain('No knowledge sources.'));
    expect(m.container.querySelector('.text-error')).toBeNull();
  });
});

// The list itself: every source with its endpoint. The initial read moves to
// the loader; the cards stay the page's render of the state's list.
describe('Knowledge sources /+page — sources', () => {
  it('lists every source with its endpoint', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(okSources(sources)));
    const m = mountComponent(Page, { data: { sources, error: null } } as never);
    cleanup.push(m.destroy);
    await vi.waitFor(() => expect(m.container.querySelectorAll('[data-testid="source-card"]').length).toBe(2));
    expect(m.container.textContent).toContain('Hive s1');
    expect(m.container.textContent).toContain('https://s1.example.com');
  });
});

// The writes stay in KnowledgeSourcesState after the move; the form is the
// page's wiring to state.add, pinned here so phase B keeps it.
describe('Knowledge sources /+page — add a source', () => {
  it('creates the source and reloads the list', async () => {
    const posts: Array<{ url: string; body: unknown }> = [];
    vi.stubGlobal('fetch', vi.fn().mockImplementation(async (url: string, init?: RequestInit) => {
      if (init?.method === 'POST') {
        posts.push({ url: String(url), body: JSON.parse(String(init.body)) });
        return { ok: true, status: 200, json: async () => source('s3') };
      }
      return okSources(sources);
    }));
    const m = mountComponent(Page, { data: { sources, error: null } } as never);
    cleanup.push(m.destroy);
    await vi.waitFor(() => expect(m.container.querySelectorAll('[data-testid="source-card"]').length).toBe(2));
    const [urlInput, nameInput] = [...m.container.querySelectorAll('.ks-input')] as HTMLInputElement[];
    urlInput.value = 'https://new.example.com';
    urlInput.dispatchEvent(new Event('input'));
    nameInput.value = 'New';
    nameInput.dispatchEvent(new Event('input'));
    (m.container.querySelector('[data-testid="add-source"]') as HTMLButtonElement).click();
    await vi.waitFor(() => expect(posts.length).toBe(1));
    expect(posts[0]?.url).toContain('/api/knowledge/sources');
    expect(posts[0]?.body).toEqual({ url: 'https://new.example.com', name: 'New' });
  });
});

// L5: the list is the loader's (+page.ts) read; the page renders it and makes
// no read of its own on mount. A failed read stays the error it was (#288).
describe('Knowledge sources /+page — the loader’s data', () => {
  it.skip('pending: L5 knowledge-sources — the loader’s data: renders the sources, without a read of its own', async () => {
    const fetchSpy = vi.fn().mockResolvedValue({ ok: false, status: 503, statusText: 'Service Unavailable' });
    vi.stubGlobal('fetch', fetchSpy);
    const m = mountComponent(Page, { data: { sources, error: null } } as never);
    cleanup.push(m.destroy);
    await vi.waitFor(() => expect(m.container.querySelectorAll('[data-testid="source-card"]').length).toBe(2));
    expect(fetchSpy).not.toHaveBeenCalled();
  });

  it.skip('pending: L5 knowledge-sources — the loader’s data: a failed loader read is the error, not a silent list', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(okSources([])));
    const m = mountComponent(Page, { data: { sources: [], error: 'Service Unavailable' } } as never);
    cleanup.push(m.destroy);
    await vi.waitFor(() =>
      expect(m.container.textContent).toContain('Service Unavailable'),
    );
  });
});
