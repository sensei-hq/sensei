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

  /** Apply a state event directly. */
  apply(event: StateEvent<T>): void;
}

// ── State event (SSE payload shape) ──────────────────────────────────────────

/**
 * Standard event shape for SSE streams.
 * The EventManager parses raw SSE data into this shape,
 * then the state store applies it via store.apply(event).
 */
export interface StateEvent<T> {
  action: 'add' | 'update' | 'remove' | 'set';
  entity: string;       // e.g. "component", "repo", "library"
  id?: string;          // for update/remove
  data?: T;             // for add/update
  items?: T[];          // for set (bulk replace)
}

// Store factory lives in stage.svelte.ts (needs Svelte $state rune).
// Re-export for convenience.
export { createStore } from './stage.svelte.js';
