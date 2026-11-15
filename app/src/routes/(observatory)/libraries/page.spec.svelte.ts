// @vitest-environment jsdom
import { describe, it, expect, afterEach, vi } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import Page from './+page.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; vi.unstubAllGlobals(); });

const libraries = [
  { id: 'r', name: 'rokkit', ecosystem: 'npm', repoCount: 2, repos: ['app'] },
  { id: 'z', name: 'zod', ecosystem: 'npm', repoCount: 1, repos: ['app'] },
];

const okLibs = (libs: unknown[]) => ({ ok: true, status: 200, json: async () => ({ total: libs.length, libs }) });

// #288: a library list that could not be read is an error with retry — not
// "No libraries indexed." The loader's read is where the list comes from once
// the fetch moves to +page.ts; the fixture carries both answers so the spec
// passes before and after with the same assertions.
describe('Libraries /+page — a failed read', () => {
  it('shows the error, not the empty state', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: false, status: 503, statusText: 'Service Unavailable' }));
    const m = mountComponent(Page, { data: { libs: [], error: 'Service Unavailable' } } as never);
    cleanup.push(m.destroy);
    await vi.waitFor(() =>
      expect(m.container.querySelector('[data-screen-state="error"]')?.textContent).toContain('Service Unavailable'),
    );
    expect(m.container.textContent).not.toContain('No libraries indexed.');
  });
});

// #288 + L5: Retry has to recover. Today it re-reads /api/libs through the
// state; after the move it must still re-read through the state (not
// invalidateAll, which would re-run every load on the route).
describe('Libraries /+page — retry after a failed read', () => {
  it('re-reads the list through the state and draws it', async () => {
    const fetchSpy = vi.fn().mockResolvedValue({ ok: false, status: 503, statusText: 'Service Unavailable' });
    vi.stubGlobal('fetch', fetchSpy);
    const m = mountComponent(Page, { data: { libs: [], error: 'Service Unavailable' } } as never);
    cleanup.push(m.destroy);
    await vi.waitFor(() => expect(m.container.querySelector('[data-action="retry"]')).not.toBeNull());

    fetchSpy.mockResolvedValue(okLibs(libraries));
    (m.container.querySelector('[data-action="retry"]') as HTMLButtonElement).click();

    await vi.waitFor(() => expect(m.container.querySelectorAll('.lib-card').length).toBe(2));
    expect(m.container.querySelector('[data-screen-state="error"]')).toBeNull();
    // #63: the retry read asks for every library, never `shared: true` only.
    const urls = fetchSpy.mock.calls.map((c) => String(c[0]));
    expect(urls.some((u) => u.includes('shared=true'))).toBe(false);
  });
});

// The honest empty: no libraries, no error. Never the error state, never a
// blank pane.
describe('Libraries /+page — nothing indexed', () => {
  it('shows the empty state, not the error state', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(okLibs([])));
    const m = mountComponent(Page, { data: { libs: [], error: null } } as never);
    cleanup.push(m.destroy);
    await vi.waitFor(() => expect(m.container.textContent).toContain('No libraries indexed.'));
    expect(m.container.querySelector('[data-screen-state="error"]')).toBeNull();
  });
});

// L5: the list is the loader's (+page.ts) read; the page renders it and makes
// no read of its own on mount. A failed read stays the error it was (#288).
describe('Libraries /+page — the loader’s data', () => {
  it('renders the libraries, without a read of its own', async () => {
    const fetchSpy = vi.fn().mockResolvedValue({ ok: false, status: 503, statusText: 'Service Unavailable' });
    vi.stubGlobal('fetch', fetchSpy);
    const m = mountComponent(Page, { data: { libs: libraries, error: null } } as never);
    cleanup.push(m.destroy);
    await vi.waitFor(() => expect(m.container.querySelectorAll('.lib-card').length).toBe(2));
    expect(m.container.textContent).toContain('rokkit');
    expect(m.container.textContent).toContain('zod');
    expect(fetchSpy).not.toHaveBeenCalled();
  });

  it('a failed loader read is the error with retry, not "No libraries indexed."', async () => {
    const m = mountComponent(Page, { data: { libs: [], error: 'Service Unavailable' } } as never);
    cleanup.push(m.destroy);
    await vi.waitFor(() =>
      expect(m.container.querySelector('[data-screen-state="error"]')?.textContent).toContain('Service Unavailable'),
    );
    expect(m.container.querySelector('[data-action="retry"]')).not.toBeNull();
    expect(m.container.textContent).not.toContain('No libraries indexed.');
  });

  it('narrows the list to the search box, ignoring case', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(okLibs(libraries)));
    const m = mountComponent(Page, { data: { libs: libraries, error: null } } as never);
    cleanup.push(m.destroy);
    await vi.waitFor(() => expect(m.container.querySelectorAll('.lib-card').length).toBe(2));
    const search = m.container.querySelector('.lib-search') as HTMLInputElement;
    search.value = 'ROKK';
    search.dispatchEvent(new Event('input'));
    await vi.waitFor(() => expect(m.container.querySelectorAll('.lib-card').length).toBe(1));
    expect(m.container.textContent).toContain('rokkit');
    expect(m.container.textContent).not.toContain('zod');
  });
});

// #341: a selected library shows what it provides — its own docs, skills and
// agents — read through the daemon. The select's reads stay in the state after
// the move, so the fetch map stays; only the initial list comes from `data`.
describe('Libraries /+page — what a library provides', () => {
  it('lists the docs, skills and agents of the selected library', async () => {
    const libs = { total: 1, libs: [{ id: 'r', name: 'rokkit', ecosystem: 'npm', repoCount: 2, repos: ['app'], packages: ['@rokkit/ui'], skillCount: 1, agentCount: 1 }] };
    const answers: Record<string, unknown> = {
      '/api/libs?': libs,
      '/api/libs/r/usage': { usage: [] },
      '/api/libs/r/skills': [{ name: 'styling', focus: 'styling', body: '# Styling with rokkit' }],
      '/api/libs/r/agents': [{ name: 'rokkit-styles-reviewer', focus: 'styles', body: '# Review' }],
      '/api/libs/r/docs': [{ title: 'Button', component: 'Button', content: '# Button' }],
    };
    vi.stubGlobal('fetch', vi.fn().mockImplementation(async (url: string) => {
      const key = Object.keys(answers).find((k) => new URL(url).pathname + (url.includes('?') && k.endsWith('?') ? '?' : '') === k);
      return key ? { ok: true, status: 200, json: async () => answers[key] } : { ok: false, status: 404, statusText: 'Not Found' };
    }));
    const m = mountComponent(Page, { data: { libs: libs.libs, error: null } } as never);
    cleanup.push(m.destroy);
    await vi.waitFor(() => expect(m.container.querySelector('.lib-card')).not.toBeNull());
    (m.container.querySelector('.lib-card') as HTMLButtonElement).click();
    await vi.waitFor(() => expect(m.container.textContent).toContain('rokkit-styles-reviewer'));
    const text = m.container.textContent ?? '';
    expect(text).toContain('Provided by the library');
    expect(text).toContain('styling');
    expect(text).toContain('Button');
    expect(text).toContain('@rokkit/ui');
  });
});
