/**
 * ReactiveStageContext<T> — base class for all stage state.
 * Every method handles both single item and array.
 */

import type { StateEvent } from './types.js';

export class ReactiveStageContext<T extends { id: string }> {
  items = $state<T[]>([]);

  constructor(initial: T[] = []) {
    this.items = [...initial];
  }

  add(data: T | T[]) {
    const incoming = Array.isArray(data) ? data : [data];
    for (const item of incoming) {
      if (!this.items.find(i => i.id === item.id)) {
        this.items = [...this.items, item];
      }
    }
  }

  update(data: T | Partial<T> | (T | Partial<T>)[]) {
    const incoming = Array.isArray(data) ? data : [data];
    let changed = false;
    for (const patch of incoming) {
      if (!patch.id) continue;
      const idx = this.items.findIndex(i => i.id === patch.id);
      if (idx >= 0) {
        this.items[idx] = { ...this.items[idx], ...patch };
        changed = true;
      }
    }
    if (changed) this.items = [...this.items];
  }

  remove(data: T | T[] | string | string[]) {
    const ids = Array.isArray(data)
      ? data.map(d => typeof d === 'string' ? d : d.id)
      : [typeof data === 'string' ? data : data.id];
    this.items = this.items.filter(i => !ids.includes(i.id));
  }

  set(data: T | T[]) {
    this.items = Array.isArray(data) ? [...data] : [data];
  }

  get(id: string): T | undefined {
    return this.items.find(i => i.id === id);
  }

  apply(event: StateEvent<T>) {
    this[event.action](event.data);
  }
}
