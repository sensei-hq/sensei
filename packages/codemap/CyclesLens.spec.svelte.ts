// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import CyclesLens from './CyclesLens.svelte';
import type { Band, Level0 } from './types';

let app: Record<string, unknown> | null = null;
let target: HTMLElement;
afterEach(() => { if (app) unmount(app); app = null; target?.remove(); });

const band = (source: string, target: string, weight: number): Band => ({ source, target, weight, kinds: {}, direction: 'level' });
const L0 = {
  level: 'module', groupBy: 'layers', coverage: { unplaced: 0 }, blocks: [],
  cycles: [{ members: ['p/a', 'p/b'] }, { members: ['p/c', 'p/d', 'p/e'] }],
  bands: [band('p/a', 'p/b', 3), band('p/b', 'p/a', 7), band('p/c', 'p/d', 5), band('p/d', 'p/e', 2), band('p/e', 'p/c', 9)],
  budget: { drawn: 0, modules: 0, bands: 0, edges: 0 }, libraries: { count: 0, top: [] },
} as unknown as Level0;

function render(data: Level0 = L0) {
  const onopen = vi.fn();
  target = document.body.appendChild(document.createElement('div'));
  app = mount(CyclesLens, { target, props: { data, zoom: 1, onopen } });
  flushSync();
  return onopen;
}

describe('CyclesLens', () => {
  it('draws each cycle on its own, the edge to cut marked', () => {
    render();
    const rings = [...target.querySelectorAll('[data-cycle]')];
    expect(rings.map((r) => r.querySelector('[data-cycle-title]')?.textContent?.trim())).toEqual([
      'Cycle 1 · 2 modules · cut p/a → p/b', 'Cycle 2 · 3 modules · cut p/d → p/e',
    ]);
    expect([...target.querySelectorAll('[data-cut]')].map((e) => e.getAttribute('data-edge'))).toEqual(['p/a>p/b', 'p/d>p/e']);
  });

  it('a member opens its module', () => {
    const onopen = render();
    (target.querySelector('[data-cycle-node="p/d"]') as SVGElement).dispatchEvent(new MouseEvent('click', { bubbles: true }));
    expect(onopen).toHaveBeenCalledWith('p/d');
  });

  it('says so when there is no cycle', () => {
    render({ ...L0, cycles: [] } as Level0);
    expect(target.textContent).toContain('No cycles');
  });
});
