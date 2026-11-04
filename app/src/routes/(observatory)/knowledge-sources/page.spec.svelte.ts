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
    // Not the error state — checked by the state marker, not a style class —
    // and no cards invented for an empty read.
    expect(m.container.querySelector('[data-screen-state="error"]')).toBeNull();
    expect(m.container.querySelectorAll('[data-testid="source-card"]').length).toBe(0);
  });
});

// The list itself: every source with its endpoint. The initial read moves to
// the loader; the cards stay the page's render of the state's list.
describe('Knowledge sources /+page — sources', () => {
  it('lists every source with its endpoint', async () => {
    const bare = source('s3', { name: '', last_seq: null, enabled: false });
    const list = [...sources, bare];
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(okSources(list)));
    const m = mountComponent(Page, { data: { sources: list, error: null } } as never);
    cleanup.push(m.destroy);
    await vi.waitFor(() => expect(m.container.querySelectorAll('[data-testid="source-card"]').length).toBe(3));
    expect(m.container.textContent).toContain('Hive s1');
    expect(m.container.textContent).toContain('https://s1.example.com');
    // L2: the card falls back to the URL for a title, and shows an absent
    // sequence and a switched-off source as absent and off. Scoped to the bare
    // card and counted: every card prints its URL in the mono line, so a plain
    // textContent check would pass even with no title fallback at all.
    const bareCard = m.container.querySelectorAll('[data-testid="source-card"]')[2]!;
    expect(bareCard.textContent!.split('https://s3.example.com').length - 1).toBe(2);
    expect(m.container.textContent).toContain('seq —');
    expect(m.container.textContent).toContain('disabled');
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
      return okSources(posts.length ? [...sources, source('s3')] : sources);
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
    // M1: the reload the title names — the re-read reaches the screen, and
    // the form clears on success.
    await vi.waitFor(() => expect(m.container.querySelectorAll('[data-testid="source-card"]').length).toBe(3));
    expect(urlInput.value).toBe('');
    expect(nameInput.value).toBe('');
  });
});

// H1: the Remove button is the page's wiring to state.remove — success
// re-reads onto the screen, failure is said aloud with the list kept.
describe('Knowledge sources /+page — remove a source', () => {
  it('deletes the source and the re-read reaches the screen', async () => {
    const deletes: string[] = [];
    vi.stubGlobal('fetch', vi.fn().mockImplementation(async (url: string, init?: RequestInit) => {
      if (init?.method === 'DELETE') {
        deletes.push(String(url));
        return { ok: true, status: 200 };
      }
      return okSources(deletes.length ? [source('s2')] : sources);
    }));
    const m = mountComponent(Page, { data: { sources, error: null } } as never);
    cleanup.push(m.destroy);
    await vi.waitFor(() => expect(m.container.querySelectorAll('[data-testid="source-card"]').length).toBe(2));
    (m.container.querySelector('[data-testid="remove-s1"]') as HTMLButtonElement).click();
    await vi.waitFor(() => expect(m.container.querySelectorAll('[data-testid="source-card"]').length).toBe(1));
    expect(deletes).toHaveLength(1);
    expect(deletes[0]).toMatch(/\/api\/knowledge\/sources\/s1$/);
    expect(m.container.textContent).toContain('Hive s2');
    expect(m.container.textContent).not.toContain('Hive s1');
  });

  it('a failed delete says so and keeps both cards', async () => {
    vi.stubGlobal('fetch', vi.fn().mockImplementation(async (url: string, init?: RequestInit) => {
      if (init?.method === 'DELETE') return { ok: false, status: 500, statusText: 'boom' };
      return okSources(sources);
    }));
    const m = mountComponent(Page, { data: { sources, error: null } } as never);
    cleanup.push(m.destroy);
    await vi.waitFor(() => expect(m.container.querySelectorAll('[data-testid="source-card"]').length).toBe(2));
    (m.container.querySelector('[data-testid="remove-s1"]') as HTMLButtonElement).click();
    await vi.waitFor(() => expect(m.container.textContent).toContain('boom'));
    expect(m.container.querySelectorAll('[data-testid="source-card"]').length).toBe(2);
  });
});

// H1: the Sync button is the page's wiring to state.sync — one POST to the
// source's sync route, then a re-read.
describe('Knowledge sources /+page — sync a source', () => {
  it('syncs the source and re-reads the list', async () => {
    const posts: string[] = [];
    let gets = 0;
    vi.stubGlobal('fetch', vi.fn().mockImplementation(async (url: string, init?: RequestInit) => {
      if (init?.method === 'POST') {
        posts.push(String(url));
        return { ok: true, status: 200, json: async () => ({}) };
      }
      gets += 1;
      return okSources(sources);
    }));
    const m = mountComponent(Page, { data: { sources, error: null } } as never);
    cleanup.push(m.destroy);
    await vi.waitFor(() => expect(m.container.querySelectorAll('[data-testid="source-card"]').length).toBe(2));
    const getsBefore = gets;
    (m.container.querySelector('[data-testid="sync-s1"]') as HTMLButtonElement).click();
    await vi.waitFor(() => expect(posts.length).toBe(1));
    expect(posts[0]).toMatch(/\/api\/knowledge\/sources\/s1\/sync$/);
    await vi.waitFor(() => expect(gets).toBeGreaterThan(getsBefore));
  });

  it('a failed sync says so', async () => {
    vi.stubGlobal('fetch', vi.fn().mockImplementation(async (url: string, init?: RequestInit) => {
      if (init?.method === 'POST') return { ok: false, status: 502, statusText: 'hive down' };
      return okSources(sources);
    }));
    const m = mountComponent(Page, { data: { sources, error: null } } as never);
    cleanup.push(m.destroy);
    await vi.waitFor(() => expect(m.container.querySelectorAll('[data-testid="source-card"]').length).toBe(2));
    (m.container.querySelector('[data-testid="sync-s1"]') as HTMLButtonElement).click();
    await vi.waitFor(() => expect(m.container.textContent).toContain('hive down'));
  });
});

// H2: this page has no Retry, so a succeeding write is the only way out of
// the error state on the screen. Phase B must keep the error in the state,
// where load() overwrites it — not render data.error past it.
describe('Knowledge sources /+page — recovery through a write', () => {
  it('a succeeding add clears the failed-read error and draws the list', async () => {
    let up = false;
    vi.stubGlobal('fetch', vi.fn().mockImplementation(async (url: string, init?: RequestInit) => {
      if (init?.method === 'POST') return { ok: true, status: 200, json: async () => source('s3') };
      return up ? okSources(sources) : { ok: false, status: 503, statusText: 'Service Unavailable' };
    }));
    const m = mountComponent(Page, { data: { sources: [], error: 'Service Unavailable' } } as never);
    cleanup.push(m.destroy);
    await vi.waitFor(() => expect(m.container.textContent).toContain('Service Unavailable'));
    up = true;
    const [urlInput, nameInput] = [...m.container.querySelectorAll('.ks-input')] as HTMLInputElement[];
    urlInput.value = 'https://new.example.com';
    urlInput.dispatchEvent(new Event('input'));
    nameInput.value = 'New';
    nameInput.dispatchEvent(new Event('input'));
    (m.container.querySelector('[data-testid="add-source"]') as HTMLButtonElement).click();
    await vi.waitFor(() => expect(m.container.querySelectorAll('[data-testid="source-card"]').length).toBe(2));
    expect(m.container.textContent).not.toContain('Service Unavailable');
  });
});

// M2: the add form's failure paths must reach the user at page level — the
// #263 name check with no POST, and a refused create that keeps the form.
describe('Knowledge sources /+page — add a source that fails', () => {
  it('#263: without a name nothing is posted and the form says so', async () => {
    const fetchSpy = vi.fn().mockImplementation(async () => okSources(sources));
    vi.stubGlobal('fetch', fetchSpy);
    const m = mountComponent(Page, { data: { sources, error: null } } as never);
    cleanup.push(m.destroy);
    await vi.waitFor(() => expect(m.container.querySelectorAll('[data-testid="source-card"]').length).toBe(2));
    const [urlInput] = [...m.container.querySelectorAll('.ks-input')] as HTMLInputElement[];
    urlInput.value = 'https://new.example.com';
    urlInput.dispatchEvent(new Event('input'));
    (m.container.querySelector('[data-testid="add-source"]') as HTMLButtonElement).click();
    await vi.waitFor(() => expect(m.container.textContent).toContain('name required'));
    expect(fetchSpy.mock.calls.some(([, init]) => (init as RequestInit | undefined)?.method === 'POST')).toBe(false);
  });

  it('a refused create keeps the form and says why', async () => {
    vi.stubGlobal('fetch', vi.fn().mockImplementation(async (url: string, init?: RequestInit) => {
      if (init?.method === 'POST') return { ok: false, status: 400, statusText: 'bad' };
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
    await vi.waitFor(() => expect(m.container.textContent).toContain('bad'));
    expect(urlInput.value).toBe('https://new.example.com');
    expect(nameInput.value).toBe('New');
  });
});

// L5: the list is the loader's (+page.ts) read; the page renders it and makes
// no read of its own on mount. A failed read stays the error it was (#288).
describe('Knowledge sources /+page — the loader’s data', () => {
  it('renders the sources, without a read of its own', async () => {
    const fetchSpy = vi.fn().mockResolvedValue({ ok: false, status: 503, statusText: 'Service Unavailable' });
    vi.stubGlobal('fetch', fetchSpy);
    const m = mountComponent(Page, { data: { sources, error: null } } as never);
    cleanup.push(m.destroy);
    await vi.waitFor(() => expect(m.container.querySelectorAll('[data-testid="source-card"]').length).toBe(2));
    expect(fetchSpy).not.toHaveBeenCalled();
  });

  it('a failed loader read is the error, not a silent list', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(okSources([])));
    const m = mountComponent(Page, { data: { sources: [], error: 'Service Unavailable' } } as never);
    cleanup.push(m.destroy);
    await vi.waitFor(() =>
      expect(m.container.textContent).toContain('Service Unavailable'),
    );
  });
});
