// @vitest-environment jsdom
import { describe, it, expect, afterEach, vi } from 'vitest';
import { flushSync } from 'svelte';
import { mountComponent } from '$lib/test-mount.js';
import { wizardState } from '$lib/wizard-state.svelte.js';
import RootsSection from './RootsSection.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => {
  cleanup.forEach((fn) => fn());
  cleanup = [];
  vi.unstubAllGlobals();
  wizardState.roots = { roots: [], newPath: '' };
});

// #304: a root the daemon refused to add left the typed path in the box and
// said nothing — the user could not tell a refusal from a slow add.
describe('RootsSection — adding a root', () => {
  it('a refused add keeps the typed path and says why', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => new Response('{}', { status: 400, statusText: 'Bad Request' })));
    wizardState.roots = { roots: [], newPath: '/nowhere' };
    const m = mountComponent(RootsSection, {});
    cleanup.push(m.destroy);

    m.container.querySelector<HTMLButtonElement>('[data-testid="root-add"]')!.click();
    await vi.waitFor(() =>
      expect(m.container.querySelector('[data-testid="root-add-error"]')?.textContent).toMatch(/400 Bad Request/),
    );
    flushSync();
    expect(m.container.querySelector<HTMLInputElement>('input.folder-input')!.value).toBe('/nowhere');
    expect(wizardState.roots.roots).toEqual([]);
  });

  it('an added root is listed and the error clears', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => new Response(JSON.stringify({ ok: true, id: 'r1', path: '/code' }), { status: 200 })));
    wizardState.roots = { roots: [], newPath: '/code' };
    const m = mountComponent(RootsSection, {});
    cleanup.push(m.destroy);

    m.container.querySelector<HTMLButtonElement>('[data-testid="root-add"]')!.click();
    await vi.waitFor(() => expect(wizardState.roots.roots.map((r) => r.id)).toEqual(['r1']));
    flushSync();
    expect(m.container.querySelector('[data-testid="root-add-error"]')).toBeNull();
    expect(m.container.querySelector<HTMLInputElement>('input.folder-input')!.value).toBe('');
  });
});

// The user (2026-10-27): a root's row showed a stray mark before its path — a
// decorative ▸ that read as a dot. The path is the row's first text.
describe('RootsSection — a root row', () => {
  it('starts with the path, nothing before it', () => {
    wizardState.roots = {
      roots: [{ id: 'r1', path: '/code', repos_found: 0, status: 'watching', excluded: [] }] as never,
      newPath: '',
    };
    const m = mountComponent(RootsSection, {});
    cleanup.push(m.destroy);
    const row = m.container.querySelector('[data-component="root-item"]') as HTMLElement;
    expect(row.textContent?.trim().startsWith('/code')).toBe(true);
  });
});
