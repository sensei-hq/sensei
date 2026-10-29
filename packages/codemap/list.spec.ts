import { describe, expect, it } from 'vitest';
import { listView } from './list';
import type { Block, CoChange, Level0, Zones } from './types';

const block = (id: string, group: string, extra: Partial<Block> = {}): Block => ({
  id, label: id, group, package: 'p', files: 3, symbols: 10,
  roles: { code: 2, test: 1, docs: 0, config: 0, generated: 0 },
  hub: false, weight_in: 0, weight_out: 0, folded: [], ...extra,
});
const L0: Level0 = {
  level: 'module', groupBy: 'layers', cycles: [{ members: ['p/b', 'p/c'] }], coverage: { unplaced: 0 },
  blocks: [
    block('p/a', 'L0', { symbols: 50, complexity: { functions: 10, complex: 3 } }),
    block('p/b', 'L1', { symbols: 20 }),
    block('p/c', 'L1', { symbols: 30, complexity: { functions: 10, complex: 1 } }),
    block('p/log', 'L1', { hub: true }),
  ],
  bands: [
    { source: 'p/a', target: 'p/b', weight: 4, kinds: {}, direction: 'up' },
    { source: 'p/b', target: 'p/c', weight: 2, kinds: {}, direction: 'level' },
    { source: 'p/c', target: 'p/b', weight: 1, kinds: {}, direction: 'level' },
  ],
  budget: { drawn: 4, modules: 4, bands: 3, edges: 7 },
  libraries: { count: 0, top: [] },
};
const ids = (v: ReturnType<typeof listView>) => v.groups.map((g) => [g.label, g.rows.map((r) => r.id)]);

describe('listView (under 640px, M17)', () => {
  it('Structure: modules by group, largest first, with their role bar and an upward badge', () => {
    const v = listView('structure', L0, null, null);
    expect(ids(v)).toEqual([['L0 · 1', ['p/a']], ['L1 · 2', ['p/c', 'p/b']]]);
    const a = v.groups[0].rows[0];
    expect(a.roles).toEqual({ code: 2, test: 1, docs: 0, config: 0, generated: 0 });
    expect(a.badges.map((b) => b.text)).toEqual(['upward']);
  });

  it('Cycles: only the modules in a cycle, badged', () => {
    const v = listView('cycles', L0, null, null);
    expect(ids(v)).toEqual([['L1 · 2', ['p/c', 'p/b']]]);
    expect(v.groups[0].rows[0].badges.map((b) => b.text)).toEqual(['cycle']);
  });

  it('Complexity: most complex first, with the share as a badge', () => {
    const v = listView('complexity', L0, null, null);
    expect(v.groups[0].rows[0].badges.map((b) => b.text)).toContain('30% over cx 10');
    expect(ids(v)[1]).toEqual(['L1 · 2', ['p/c', 'p/b']]);
  });

  it('Hidden coupling: modules with a co-change partner, naming it', () => {
    const cc: CoChange = { hidden: [{ a: 'p/a', b: 'p/c', together: 7, strength: 0.7 }], pairs: 1, commits: 40, minTogether: 5 };
    const v = listView('coupling', L0, cc, null);
    expect(ids(v)).toEqual([['L0 · 1', ['p/a']], ['L1 · 1', ['p/c']]]);
    expect(v.groups[0].rows[0].badges.map((b) => b.text)).toEqual(['with p/c']);
  });

  it('Zones: ranked by distance from the main sequence', () => {
    const z: Zones = {
      meanDistance: 0.5, coverage: { points: 2, unplaceable: 0 },
      points: [
        { component: 'p/a', types: 1, abstractTypes: 0, ca: 1, ce: 0, abstractness: 0, instability: 0, distance: 1, zone: 'pain' },
        { component: 'p/b', types: 1, abstractTypes: 0, ca: 0, ce: 1, abstractness: 0, instability: 0.8, distance: 0.2, zone: 'main' },
      ],
    };
    const v = listView('zones', L0, null, z);
    expect(ids(v)).toEqual([['Furthest from the main sequence', ['p/a', 'p/b']]]);
    expect(v.groups[0].rows[0].badges.map((b) => b.text)).toEqual(['zone of pain']);
  });

  it('Matrix: group → group dependencies, heaviest first, with upward counted', () => {
    const v = listView('matrix', L0, null, null);
    expect(v.groups[0].rows.map((r) => [r.id, r.sub, r.badges.map((b) => b.text)])).toEqual([
      ['L0 → L1', '4 calls + imports', ['1 upward']],
      ['L1 → L1', '3 calls + imports', []],
    ]);
  });
});
