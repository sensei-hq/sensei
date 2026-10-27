import { describe, expect, it } from 'vitest';
import { couplingBands, cycleBands, matrix, zonesView } from './lenses';
import type { Band, Block, Level0, ZonePoint } from './types';

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

describe('zonesView', () => {
  const pt = (component: string, distance: number, zone: ZonePoint['zone']): ZonePoint => ({
    component, types: 1, abstractTypes: 0, ca: 1, ce: 1, abstractness: 0, instability: 0.5, distance, zone,
  });
  it('places what it can, counts what it cannot, and lists the furthest first', () => {
    const v = zonesView({ points: [pt('a', 0.1, 'main'), pt('b', 0.9, 'pain'), pt('c', 0.5, null), pt('d', 0.7, 'risk')],
      meanDistance: 0.5, coverage: { points: 4, unplaceable: 1 } }, 2);
    expect(v.placed.map((p) => p.component)).toEqual(['a', 'b', 'd']);
    expect(v.unplaceable).toBe(1);
    expect(v.furthest.map((p) => p.component)).toEqual(['b', 'd']);
    expect(v.byZone).toEqual({ main: 1, pain: 1, useless: 0, risk: 1 });
  });
});

describe('couplingBands', () => {
  it('draws each hidden coupling between the blocks that hold its modules', () => {
    const folded: Level0 = { ...l0, blocks: [...l0.blocks, block('fold:L2', 'L2'), ] };
    folded.blocks[folded.blocks.length - 1].folded = ['x', 'y'];
    const bands = couplingBands(folded, [
      { a: 'top', b: 'x', together: 6, strength: 1 },
      { a: 'top', b: 'y', together: 4, strength: 0.5 },
      { a: 'x', b: 'y', together: 9, strength: 1 },
      { a: 'top', b: 'gone', together: 9, strength: 1 },
    ]);
    expect(bands.map((b) => [b.source, b.target, b.weight])).toEqual([['top', 'fold:L2', 10]]);
    expect(bands[0].direction).toBeNull();
  });
});
