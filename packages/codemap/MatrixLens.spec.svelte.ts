// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import MatrixLens from './MatrixLens.svelte';
import type { Block, Level0 } from './types';

const block = (id: string, group: string): Block => ({
  id, label: id, group, package: 'p', files: 1, symbols: 10,
  roles: { code: 1, test: 0, docs: 0, config: 0, generated: 0 },
  hub: false, weight_in: 0, weight_out: 0, folded: [],
});
const L0: Level0 = {
  level: 'module', groupBy: 'layers', cycles: [], coverage: { unplaced: 0 },
  blocks: [block('p/a', 'L0'), block('p/b', 'L1')],
  bands: [{ source: 'p/b', target: 'p/a', weight: 4, kinds: {}, direction: 'up' }],
  budget: { drawn: 2, modules: 2, bands: 1, edges: 4 }, libraries: { count: 0, top: [] },
};

let app: Record<string, unknown> | null = null;
let target: HTMLElement;
afterEach(() => { if (app) unmount(app); app = null; target?.remove(); });

describe('MatrixLens', () => {
  it('names the columns along the top and the rows down the side, both held in view while it scrolls', () => {
    const onopen = vi.fn();
    target = document.body.appendChild(document.createElement('div'));
    app = mount(MatrixLens, { target, props: { data: L0, zoom: 1, onopen } });
    flushSync();
    const cols = [...target.querySelectorAll('[data-testid="matrix-col"]')] as HTMLElement[];
    const rows = [...target.querySelectorAll('[data-testid="matrix-row"]')] as HTMLElement[];
    expect([cols.map((c) => c.textContent?.trim()), rows.map((r) => r.textContent?.trim())]).toEqual([['a', 'b'], ['a', 'b']]);
    expect([cols[0].className.includes('sticky'), rows[0].className.includes('sticky')]).toEqual([true, true]);
    const cell = target.querySelector('[data-cell="p/b>p/a"]') as HTMLElement;
    expect(cell.dataset.upward).toBe('true');
    (rows[1].querySelector('button') as HTMLButtonElement).click();
    expect(onopen).toHaveBeenCalledWith('p/b');
  });
});
