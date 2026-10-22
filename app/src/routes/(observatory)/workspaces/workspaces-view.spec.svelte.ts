// @vitest-environment jsdom
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { WorkspacesViewState } from './workspaces-view.svelte.js';
import { STORAGE_KEYS } from '$lib/storage-keys.js';

// A working Map-backed localStorage. The runner's experimental `localStorage`
// global is a non-functional stub, so mock it explicitly (as appstate.spec
// does) to get a deterministic store for the persistence assertions.
const store = new Map<string, string>();
vi.stubGlobal('localStorage', {
  getItem: (key: string) => store.get(key) ?? null,
  setItem: (key: string, value: string) => store.set(key, value),
  removeItem: (key: string) => store.delete(key),
  clear: () => store.clear(),
});

beforeEach(() => store.clear());

describe('WorkspacesViewState', () => {
  it('defaults to grid when nothing is stored', () => {
    expect(new WorkspacesViewState().view).toBe('grid');
  });

  it('setView persists the choice to localStorage', () => {
    const s = new WorkspacesViewState();
    s.setView('list');
    expect(s.view).toBe('list');
    expect(localStorage.getItem(STORAGE_KEYS.workspacesView)).toBe('list');
  });

  it('a fresh instance reads the persisted view (survives a reload)', () => {
    localStorage.setItem(STORAGE_KEYS.workspacesView, 'list');
    expect(new WorkspacesViewState().view).toBe('list');
  });

  it('an invalid stored value falls back to grid', () => {
    localStorage.setItem(STORAGE_KEYS.workspacesView, 'mosaic');
    expect(new WorkspacesViewState().view).toBe('grid');
  });

  it('toggleView flips grid ↔ list and persists both directions', () => {
    const s = new WorkspacesViewState();
    s.toggleView();
    expect(s.view).toBe('list');
    expect(localStorage.getItem(STORAGE_KEYS.workspacesView)).toBe('list');
    s.toggleView();
    expect(s.view).toBe('grid');
    expect(localStorage.getItem(STORAGE_KEYS.workspacesView)).toBe('grid');
  });

  it('toggling the view does not clear the filter or the query', () => {
    const s = new WorkspacesViewState();
    s.setFilter('archived');
    s.query = 'sen';
    s.toggleView();
    expect(s.filter).toBe('archived');
    expect(s.query).toBe('sen');
  });

  it('setView leaves the filter and query untouched', () => {
    const s = new WorkspacesViewState();
    s.setFilter('active');
    s.query = 'rok';
    s.setView('list');
    expect(s.filter).toBe('active');
    expect(s.query).toBe('rok');
  });

  it('clearQuery empties the search', () => {
    const s = new WorkspacesViewState();
    s.query = 'x';
    s.clearQuery();
    expect(s.query).toBe('');
  });
});
