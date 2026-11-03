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

  it('the controls band and the side panel are each toggled and remembered on their own', () => {
    const h = new DiagramHeaderState();
    expect([h.controls, h.side]).toEqual([true, true]);
    h.toggleControls();
    expect([new DiagramHeaderState().controls, new DiagramHeaderState().side]).toEqual([false, true]);
    new DiagramHeaderState().toggleSide();
    expect(new DiagramHeaderState().side).toBe(false);
  });

  it('the stats band is shown by default, and hiding it is remembered apart from the info', () => {
    const h = new DiagramHeaderState();
    expect(h.stats).toBe(true);
    h.toggleStats();
    expect([new DiagramHeaderState().stats, new DiagramHeaderState().open]).toEqual([false, true]);
  });
});
