import { beforeEach, describe, expect, it, vi } from 'vitest';
import { DiagramHeaderState } from './diagram-header.svelte.js';

// A working Map-backed localStorage: the runner's own is a non-functional stub.
const store = new Map<string, string>();
vi.stubGlobal('localStorage', {
  getItem: (k: string) => store.get(k) ?? null,
  setItem: (k: string, v: string) => void store.set(k, v),
  removeItem: (k: string) => void store.delete(k),
  clear: () => store.clear(),
});

// "What it's for" is collapsible and the choice is remembered (M17).
describe('DiagramHeaderState', () => {
  beforeEach(() => store.clear());

  it('opens by default, and a closed header stays closed in the next window', () => {
    const first = new DiagramHeaderState();
    expect(first.open).toBe(true);
    first.toggle();
    expect(first.open).toBe(false);
    expect(new DiagramHeaderState().open).toBe(false);
    new DiagramHeaderState().toggle();
    expect(new DiagramHeaderState().open).toBe(true);
  });
});
