// Observatory · Workspaces — index UI state.
//
// Owns the three pieces of view state the screen carries between renders:
// the grid/list `view` (persisted to localStorage so a return visit lands
// in the same layout), the status `filter` chip, and the search `query`.
// Persisting the view here — not in the component — keeps the toggle a pure
// preference: flipping it re-lays the already-loaded data without touching
// the filter or search, and never refetches.
import { STORAGE_KEYS, browserStorage as storage } from '$lib/storage-keys.js';

export type WorkspaceView = 'grid' | 'list';
export type WorkspaceFilter = 'all' | 'active' | 'dormant' | 'archived';

const DEFAULT_VIEW: WorkspaceView = 'grid';

function readView(): WorkspaceView {
  const stored = storage()?.getItem(STORAGE_KEYS.workspacesView);
  return stored === 'list' || stored === 'grid' ? stored : DEFAULT_VIEW;
}

function writeView(view: WorkspaceView): void {
  storage()?.setItem(STORAGE_KEYS.workspacesView, view);
}

export class WorkspacesViewState {
  /** Grid (default) or list. Hydrated from localStorage on construction. */
  view = $state<WorkspaceView>(readView());
  /** Active filter chip. */
  filter = $state<WorkspaceFilter>('all');
  /** Case-insensitive search over name + client. */
  query = $state('');

  /** Set the view and persist it. Does not touch filter or query. */
  setView(view: WorkspaceView): void {
    this.view = view;
    writeView(view);
  }

  /** Flip grid ↔ list, persisting the result. */
  toggleView(): void {
    this.setView(this.view === 'grid' ? 'list' : 'grid');
  }

  setFilter(filter: WorkspaceFilter): void {
    this.filter = filter;
  }

  clearQuery(): void {
    this.query = '';
  }
}

/** Screen singleton — survives in-app navigation so the toggle, filter, and
 *  search hold while the user drills into a workspace and comes back. A real
 *  browser reload re-reads the persisted view from localStorage. */
export const workspacesView = new WorkspacesViewState();
