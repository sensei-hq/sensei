/**
 * Svelte 5 reactive state store factory.
 * Uses $state rune for automatic reactivity in Svelte components.
 * Same interface as createStore in stage.ts, but reactive.
 */

import type { StateStore } from './stage.js';

export function createReactiveStore<T extends { id: string }>(initial: T[] = []): StateStore<T> {
  let _items = $state<T[]>(initial);

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
