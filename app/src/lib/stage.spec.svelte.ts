/**
 * Tests for the Stage pattern foundation — StateReactiveStageContext CRUD and event dispatch.
 */
import { describe, it, expect } from 'vitest';
import { ReactiveStageContext } from './stage.svelte.js';
// Alias for brevity in tests
const Ctx = ReactiveStageContext;
import type { StateEvent } from './types.js';

// ── Test entity ──────────────────────────────────────────────────────────────

interface Item {
  id: string;
  name: string;
  status: string;
}

function item(id: string, name: string, status = 'pending'): Item {
  return { id, name, status };
}

// ── ReactiveStageContext constructor ─────────────────────────────────────────────────────────

describe('ReactiveStageContext', () => {
  it('starts empty by default', () => {
    const store = new Ctx<Item>();
    expect(store.items).toEqual([]);
  });

  it('starts with initial items', () => {
    const store = new Ctx<Item>([item('1', 'first')]);
    expect(store.items).toHaveLength(1);
    expect(store.items[0].name).toBe('first');
  });
});

// ── add ──────────────────────────────────────────────────────────────────────

describe('add', () => {
  it('adds an item', () => {
    const store = new Ctx<Item>();
    store.add(item('1', 'alpha'));
    expect(store.items).toHaveLength(1);
    expect(store.items[0].id).toBe('1');
  });

  it('does not add duplicate ids', () => {
    const store = new Ctx<Item>([item('1', 'alpha')]);
    store.add(item('1', 'alpha-dupe'));
    expect(store.items).toHaveLength(1);
    expect(store.items[0].name).toBe('alpha');
  });

  it('adds multiple distinct items', () => {
    const store = new Ctx<Item>();
    store.add(item('1', 'alpha'));
    store.add(item('2', 'beta'));
    expect(store.items).toHaveLength(2);
  });
});

// ── update ───────────────────────────────────────────────────────────────────

describe('update', () => {
  it('updates an existing item', () => {
    const store = new Ctx<Item>([item('1', 'alpha', 'pending')]);
    store.update({ id: '1', status: 'ready' } as Item);
    expect(store.items[0].status).toBe('ready');
    expect(store.items[0].name).toBe('alpha'); // unchanged fields preserved
  });

  it('ignores update for nonexistent id', () => {
    const store = new Ctx<Item>([item('1', 'alpha')]);
    store.update({ id: '999', status: 'ready' } as Item);
    expect(store.items).toHaveLength(1);
    expect(store.items[0].status).toBe('pending');
  });
});

// ── remove ───────────────────────────────────────────────────────────────────

describe('remove', () => {
  it('removes an item by id', () => {
    const store = new Ctx<Item>([item('1', 'alpha'), item('2', 'beta')]);
    store.remove('1');
    expect(store.items).toHaveLength(1);
    expect(store.items[0].id).toBe('2');
  });

  it('ignores remove for nonexistent id', () => {
    const store = new Ctx<Item>([item('1', 'alpha')]);
    store.remove('999');
    expect(store.items).toHaveLength(1);
  });
});

// ── set ──────────────────────────────────────────────────────────────────────

describe('set', () => {
  it('replaces all items', () => {
    const store = new Ctx<Item>([item('1', 'old')]);
    store.set([item('2', 'new'), item('3', 'newer')]);
    expect(store.items).toHaveLength(2);
    expect(store.items[0].id).toBe('2');
  });

  it('clears items with empty array', () => {
    const store = new Ctx<Item>([item('1', 'alpha')]);
    store.set([]);
    expect(store.items).toHaveLength(0);
  });
});

// ── get ──────────────────────────────────────────────────────────────────────

describe('get', () => {
  it('finds item by id', () => {
    const store = new Ctx<Item>([item('1', 'alpha'), item('2', 'beta')]);
    expect(store.get('2')?.name).toBe('beta');
  });

  it('returns undefined for nonexistent id', () => {
    const store = new Ctx<Item>([item('1', 'alpha')]);
    expect(store.get('999')).toBeUndefined();
  });
});

// ── store.apply ──────────────────────────────────────────────────────────────

describe('store.apply', () => {
  it('dispatches add event', () => {
    const store = new Ctx<Item>();
    store.apply({ action: 'add', entity: 'item', data: item('1', 'alpha') });
    expect(store.items).toHaveLength(1);
  });

  it('dispatches update event', () => {
    const store = new Ctx<Item>([item('1', 'alpha', 'pending')]);
    store.apply({ action: 'update', entity: 'item', data: { id: '1', name: 'alpha', status: 'ready' } });
    expect(store.items[0].status).toBe('ready');
  });

  it('dispatches remove event', () => {
    const store = new Ctx<Item>([item('1', 'alpha')]);
    store.apply({ action: 'remove', entity: 'item', data: item('1', '') });
    expect(store.items).toHaveLength(0);
  });

  it('dispatches set event', () => {
    const store = new Ctx<Item>([item('1', 'old')]);
    store.apply({ action: 'set', entity: 'item', data: [item('2', 'new')] });
    expect(store.items).toHaveLength(1);
    expect(store.items[0].id).toBe('2');
  });

  it('update applies when data has id', () => {
    const store = new Ctx<Item>([item('1', 'alpha')]);
    store.apply({ action: 'update', entity: 'item', data: item('1', 'changed') });
    expect(store.items[0].name).toBe('changed');
  });

  it('add handles array data', () => {
    const store = new Ctx<Item>();
    store.apply({ action: 'add', entity: 'item', data: [item('1', 'a'), item('2', 'b')] });
    expect(store.items).toHaveLength(2);
  });

  it('remove handles array data', () => {
    const store = new Ctx<Item>([item('1', 'a'), item('2', 'b'), item('3', 'c')]);
    store.apply({ action: 'remove', entity: 'item', data: [item('1', ''), item('3', '')] });
    expect(store.items).toHaveLength(1);
    expect(store.items[0].id).toBe('2');
  });
});

// ── End-to-end: simulate SSE event stream ────────────────────────────────────

describe('SSE simulation', () => {
  it('applies a sequence of events', () => {
    const store = new Ctx<Item>();

    // Simulate a stream of events
    const events: StateEvent<Item>[] = [
      { action: 'add', entity: 'item', data: item('1', 'homebrew', 'detecting') },
      { action: 'add', entity: 'item', data: item('2', 'postgresql', 'detecting') },
      { action: 'add', entity: 'item', data: item('3', 'ollama', 'detecting') },
      { action: 'update', entity: 'item', data: item('1', 'homebrew', 'ready') },
      { action: 'update', entity: 'item', data: item('2', 'postgresql', 'installing') },
      { action: 'update', entity: 'item', data: item('2', 'postgresql', 'ready') },
      { action: 'update', entity: 'item', data: item('3', 'ollama', 'ready') },
    ];

    for (const event of events) {
      store.apply(event);
    }

    expect(store.items).toHaveLength(3);
    expect(store.items.every(i => i.status === 'ready')).toBe(true);
  });

  it('handles remove mid-stream', () => {
    const store = new Ctx<Item>([
      item('1', 'alpha', 'ready'),
      item('2', 'beta', 'ready'),
      item('3', 'gamma', 'failed'),
    ]);

    store.apply({ action: 'remove', entity: 'item', data: item('3', '') });
    store.apply({ action: 'add', entity: 'item', data: item('4', 'delta', 'ready') });

    expect(store.items).toHaveLength(3);
    expect(store.get('3')).toBeUndefined();
    expect(store.get('4')?.status).toBe('ready');
  });
});
