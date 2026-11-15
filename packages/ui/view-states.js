// packages/ui/view-states.js — where a screen's open bands are kept (D87 §4, §8).
//
// rokkit's ScreenShell is layout only: it hands the host the whole panel state
// after each toggle and persists nothing. The host provides a store over its
// own config — the desktop app's `ui.views`, the dōjō's choice — and every
// ShellScreen below reads and writes it here, so no screen decides for itself
// and no screen writes localStorage.
//
// A store is `{ get(viewId), set(viewId, next), error?() }`: `get` returns the
// view's remembered panels (undefined on a first visit — every band open), `set`
// keeps the whole state and rejects when it cannot, and `error` says why the
// stored views cannot be read. With no provider a screen is in memory alone:
// it toggles, and nothing is claimed to be kept.
//
// `Symbol.for`, not `Symbol`: the key must be the same even if a bundler ever
// loads this module twice (once through `@sensei/ui`, once by path).

import { getContext, setContext } from 'svelte';

/**
 * @typedef {'info' | 'stats' | 'filters' | 'side'} PanelId
 * @typedef {Partial<Record<PanelId, boolean>>} PanelState
 * @typedef {{
 *   get(viewId: string): PanelState | undefined,
 *   set(viewId: string, next: PanelState): Promise<void> | void,
 *   error?(): string | null,
 * }} ViewStateStore
 */

const VIEW_STATES_CONTEXT = Symbol.for('sensei.view-states');

/**
 * Provide the view-panel store to every shell below. Call once, during the root
 * layout's init.
 * @param {ViewStateStore} store
 * @returns {void}
 */
export function setViewStates(store) {
  setContext(VIEW_STATES_CONTEXT, store);
}

/**
 * The store provided above, or null when nothing provides one. Call during
 * component init; the store's reads stay reactive.
 * @returns {ViewStateStore | null}
 */
export function useViewStates() {
  /** @type {ViewStateStore | undefined} */
  const store = getContext(VIEW_STATES_CONTEXT);
  return store ?? null;
}
