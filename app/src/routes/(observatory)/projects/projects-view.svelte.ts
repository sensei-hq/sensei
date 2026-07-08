// Observatory · Projects — index UI state.
//
// Owns the three pieces of view state the screen carries between renders:
// the grid/list `view` (persisted to localStorage so a return visit lands
// in the same layout), the status `filter` chip, and the search `query`.
// Persisting the view here — not in the component — keeps the toggle a pure
// preference: flipping it re-lays the already-loaded data without touching
// the filter or search, and never refetches.
import { STORAGE_KEYS } from '$lib/storage-keys.js';

export type ProjectView = 'grid' | 'list';
export type ProjectFilter = 'all' | 'active' | 'dormant' | 'archived';

const DEFAULT_VIEW: ProjectView = 'grid';

/** The active Storage, or null when there isn't a usable one. Feature-detects
 *  the methods rather than just `typeof localStorage` so a build-time / SSR
 *  context — or a test runner whose experimental `localStorage` global is a
 *  non-functional stub — resolves cleanly instead of throwing. */
function storage(): Storage | null {
  const ls = (globalThis as { localStorage?: Storage }).localStorage;
  return ls && typeof ls.getItem === 'function' && typeof ls.setItem === 'function' ? ls : null;
}

function readView(): ProjectView {
  const stored = storage()?.getItem(STORAGE_KEYS.projectsView);
  return stored === 'list' || stored === 'grid' ? stored : DEFAULT_VIEW;
}

function writeView(view: ProjectView): void {
  storage()?.setItem(STORAGE_KEYS.projectsView, view);
}

export class ProjectsViewState {
  /** Grid (default) or list. Hydrated from localStorage on construction. */
  view = $state<ProjectView>(readView());
  /** Active filter chip. */
  filter = $state<ProjectFilter>('all');
  /** Case-insensitive search over name + client. */
  query = $state('');

  /** Set the view and persist it. Does not touch filter or query. */
  setView(view: ProjectView): void {
    this.view = view;
    writeView(view);
  }

  /** Flip grid ↔ list, persisting the result. */
  toggleView(): void {
    this.setView(this.view === 'grid' ? 'list' : 'grid');
  }

  setFilter(filter: ProjectFilter): void {
    this.filter = filter;
  }

  clearQuery(): void {
    this.query = '';
  }
}

/** Screen singleton — survives in-app navigation so the toggle, filter, and
 *  search hold while the user drills into a project and comes back. A real
 *  browser reload re-reads the persisted view from localStorage. */
export const projectsView = new ProjectsViewState();
