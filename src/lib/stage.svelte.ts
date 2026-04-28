/**
 * Reactive state store factory — Svelte 5 $state backed.
 * Single implementation used by both components and tests.
 */

import type { StateStore, StateEvent } from './stage.js';

export function createStore<T extends { id: string }>(initial: T[] = []): StateStore<T> {
  let _items = $state<T[]>([...initial]);

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

    apply(event: StateEvent<T>) {
      switch (event.action) {
        case 'add':    if (event.data) this.add(event.data); break;
        case 'update': if (event.id && event.data) this.update(event.id, event.data); break;
        case 'remove': if (event.id) this.remove(event.id); break;
        case 'set':    if (event.items) this.set(event.items); break;
      }
    },
  };
}
