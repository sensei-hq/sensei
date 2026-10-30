import { describe, expect, it } from 'vitest';
import { takeaways } from './takeaways';
import type { Block, CoChange, Level0, Zones } from './types';

const block = (id: string, extra: Partial<Block> = {}): Block => ({
  id, label: id, group: 'L0', package: 'p', files: 3, symbols: 10,
  roles: { code: 3, test: 0, docs: 0, config: 0, generated: 0 },
  hub: false, weight_in: 0, weight_out: 0, folded: [], ...extra,
});
const L0: Level0 = {
  level: 'module', groupBy: 'layers', coverage: { unplaced: 0 },
  cycles: [{ members: ['p/a', 'p/b'] }, { members: ['p/c', 'p/d', 'p/e'] }],
  blocks: [block('p/a', { complexity: { functions: 10, complex: 3 } }), block('p/b'), block('p/c')],
  bands: [
    { source: 'p/c', target: 'p/a', weight: 9, kinds: {}, direction: 'up' },
    { source: 'p/d', target: 'p/b', weight: 2, kinds: {}, direction: 'up' },
    { source: 'p/a', target: 'p/c', weight: 4, kinds: {}, direction: 'skip' },
    { source: 'p/b', target: 'p/a', weight: 7, kinds: {}, direction: 'down' },
  ],
  budget: { drawn: 3, modules: 5, bands: 4, edges: 22 },
  libraries: { count: 0, top: [] },
};

describe('takeaways', () => {
  it('leads with the cycles, then the upward dependencies heaviest first', () => {
    const t = takeaways('structure', L0, null, null);
    expect(t.map((x) => x.title)).toEqual(['Break 2 cycles', '2 upward dependencies', '1 layer skip']);
    expect(t[0].items.map((i) => i.text)).toEqual(['p/a ⇄ p/b', 'p/c ⇄ p/d ⇄ p/e']);
    expect(t[1].items.map((i) => [i.text, i.value, i.target])).toEqual([
      ['p/c → p/a', '9', 'p/c'],
      ['p/d → p/b', '2', 'p/d'],
    ]);
  });

  it('Zones: the most depended-on modules in the zone of pain, and abstractions nothing uses', () => {
    const z: Zones = {
      meanDistance: 0.3, coverage: { points: 3, unplaceable: 0 },
      points: [
        { component: 'p/config', types: 2, abstractTypes: 0, ca: 19, ce: 5, abstractness: 0, instability: 0.21, distance: 0.79, zone: 'pain' },
        { component: 'p/model', types: 5, abstractTypes: 0, ca: 7, ce: 3, abstractness: 0, instability: 0.3, distance: 0.7, zone: 'pain' },
        { component: 'p/ports', types: 4, abstractTypes: 4, ca: 0, ce: 2, abstractness: 1, instability: 1, distance: 1, zone: 'useless' },
      ],
    };
    const t = takeaways('zones', L0, null, z);
    expect(t.map((x) => x.title)).toEqual(['Break 2 cycles', '2 upward dependencies', '2 modules in the zone of pain', '1 abstraction nothing uses']);
    expect(t[2].items.map((i) => [i.text, i.value])).toEqual([['p/config', '19 dependents'], ['p/model', '7 dependents']]);
  });

  it('Complexity: the modules over 20% first; Hidden coupling: the strongest pairs', () => {
    expect(takeaways('complexity', L0, null, null)[2]).toMatchObject({ title: '1 module over 20% complex', items: [{ text: 'p/a', value: '30%' }] });
    const cc: CoChange = { hidden: [{ a: 'p/a', b: 'p/c', together: 7, strength: 0.7 }], pairs: 1, commits: 40, minTogether: 5 };
    expect(takeaways('coupling', L0, cc, null)[2]).toMatchObject({ title: '1 hidden coupling', items: [{ text: 'p/a ⇢ p/c', value: '7 commits' }] });
  });

  it('a clean map says there is nothing to fix rather than an empty panel', () => {
    const clean = { ...L0, cycles: [], bands: [] };
    expect(takeaways('structure', clean, null, null)).toEqual([]);
  });
});
