/**
 * Base state store class — all screen stores extend this.
 * Uses Svelte 5 $state for reactivity. Implements StateStore<T> interface.
 */

import type { StateEvent } from './types.js';

export class ReactiveStageContext<T extends { id: string }> {
  items = $state<T[]>([]);

  constructor(initial: T[] = []) {
    this.items = [...initial];
  }

  add(item: T) {
    if (!this.items.find(i => i.id === item.id)) {
      this.items = [...this.items, item];
    }
  }

  update(id: string, patch: Partial<T>) {
    const idx = this.items.findIndex(i => i.id === id);
    if (idx >= 0) {
      this.items[idx] = { ...this.items[idx], ...patch };
      this.items = [...this.items];
    }
  }

  remove(id: string) {
    this.items = this.items.filter(i => i.id !== id);
  }

  set(items: T[]) {
    this.items = [...items];
  }

  get(id: string): T | undefined {
    return this.items.find(i => i.id === id);
  }

  apply(event: StateEvent<T>) {
    switch (event.action) {
      case 'add':    if (event.data && !Array.isArray(event.data)) this.add(event.data); break;
      case 'update': if (event.id && event.data && !Array.isArray(event.data)) this.update(event.id, event.data); break;
      case 'remove': if (event.id) this.remove(event.id); break;
      case 'set':    if (Array.isArray(event.data)) this.set(event.data); break;
    }
  }
}
