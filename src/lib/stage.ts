/**
 * Stage pattern — foundation for multi-step screens.
 *
 * A Stage is a self-contained unit: metadata + state store + optional event source + component.
 * Multi-step screens are arrays of stages. The shell renders the current stage's component
 * and navigates when canAdvance() returns true.
 */

import type { EventManager } from './events.js';
import type { Component } from 'svelte';

// ── Stage definition ─────────────────────────────────────────────────────────

export interface Stage {
  id: string;
  title: string;
  icon: string;               // kanji character
  description: string;
  watermark?: boolean;         // show faded kanji in background
  component: Component;        // Svelte component to render
  canAdvance: () => boolean;   // gate to next stage
  load?: () => Promise<void>;  // one-shot data fetch on stage enter
  source?: EventManager<any>;  // SSE subscription (connect on enter, disconnect on leave)
}

// ── State store interface ────────────────────────────────────────────────────

/**
 * Standard CRUD interface for state stores.
 * EventManager dispatches events using this interface.
 * Every screen's state store implements this.
 */
export interface StateStore<T extends { id: string }> {
  /** All items. */
  items: T[];

  /** Add an item. */
  add(item: T): void;

  /** Update an item by id. Merges partial data. */
  update(id: string, patch: Partial<T>): void;

  /** Remove an item by id. */
  remove(id: string): void;

  /** Replace all items (bulk set). */
  set(items: T[]): void;

  /** Find an item by id. */
  get(id: string): T | undefined;
}

// ── State event (SSE payload shape) ──────────────────────────────────────────

/**
 * Standard event shape for SSE streams.
 * The EventManager parses raw SSE data into this shape,
 * then the state store applies the corresponding operation.
 */
export interface StateEvent<T> {
  action: 'add' | 'update' | 'remove' | 'set';
  entity: string;       // e.g. "component", "repo", "library"
  id?: string;          // for update/remove
  data?: T;             // for add/update
  items?: T[];          // for set (bulk replace)
}

/**
 * Apply a state event to a store. Generic dispatcher.
 */
export function applyEvent<T extends { id: string }>(
  store: StateStore<T>,
  event: StateEvent<T>,
): void {
  switch (event.action) {
    case 'add':
      if (event.data) store.add(event.data);
      break;
    case 'update':
      if (event.id && event.data) store.update(event.id, event.data);
      break;
    case 'remove':
      if (event.id) store.remove(event.id);
      break;
    case 'set':
      if (event.items) store.set(event.items);
      break;
  }
}

// ── Create a plain (non-reactive) state store ────────────────────────────────
//
// For Svelte 5 reactive stores, use createReactiveStore from stage.svelte.ts.
// This plain version is used in tests and non-Svelte contexts.

export function createStore<T extends { id: string }>(initial: T[] = []): StateStore<T> {
  let _items: T[] = [...initial];

  return {
    get items() { return _items; },

    add(item: T) {
      if (!_items.find(i => i.id === item.id)) {
        _items = [..._items, item];
      }
    },

    update(id: string, patch: Partial<T>) {
      const idx = _items.findIndex(i => i.id === id);
      if (idx >= 0) {
        _items[idx] = { ..._items[idx], ...patch };
        _items = [..._items];
      }
    },

    remove(id: string) {
      _items = _items.filter(i => i.id !== id);
    },

    set(items: T[]) {
      _items = [...items];
    },

    get(id: string): T | undefined {
      return _items.find(i => i.id === id);
    },
  };
}
