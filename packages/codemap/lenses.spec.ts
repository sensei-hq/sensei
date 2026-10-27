import { describe, expect, it } from 'vitest';
import { cycleBands, matrix } from './lenses';
import type { Band, Block, Level0 } from './types';

const block = (id: string, group: string, symbols = 1): Block => ({
  id, label: id, group, files: 1, symbols, hub: false, weight_in: 0, weight_out: 0, folded: [],
});
const band = (source: string, target: string, direction: Band['direction'], weight = 1): Band => ({
  source, target, weight, kinds: {}, direction,
});
const l0: Level0 = {
  level: 'module', groupBy: 'layers', coverage: { unplaced: 0 },
  blocks: [block('top', 'L0'), block('a', 'L1'), block('b', 'L1'), block('base', 'L2')],
  bands: [band('top', 'a', 'down'), band('a', 'b', 'level'), band('b', 'a', 'level'), band('base', 'top', 'up', 3)],
  cycles: [{ members: ['a', 'b'] }],
  budget: { drawn: 4, modules: 4, bands: 4, edges: 6 },
};

describe('lenses', () => {
  it('Cycles keeps only the bands inside a cycle', () => {
    expect(cycleBands(l0).map((b) => `${b.source}>${b.target}`)).toEqual(['a>b', 'b>a']);
  });

  it('the Matrix puts every upward dependency below the diagonal', () => {
    const m = matrix(l0);
    expect(m.ids).toEqual(['top', 'a', 'b', 'base']);
    const [i, j] = [m.ids.indexOf('base'), m.ids.indexOf('top')];
    expect([m.weight[i][j], m.upward[i][j], i > j]).toEqual([3, true, true]);
    expect(m.max).toBe(3);
  });
});
