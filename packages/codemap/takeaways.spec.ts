import { describe, expect, it } from 'vitest';
import { takeaways, takeawaysMarkdown } from './takeaways';
import type { Block, CoChange, Level0, Zones } from './types';

const block = (id: string, extra: Partial<Block> = {}): Block => ({
  id, label: id, group: 'L0', package: 'p', files: 3, symbols: 10,
  roles: { code: 3, test: 0, docs: 0, config: 0, generated: 0 },
  hub: false, weight_in: 0, weight_out: 0, folded: [], ...extra,
});
const L0: Level0 = {
  level: 'module', groupBy: 'layers', coverage: { unplaced: 0 },
  cycles: [{ members: ['p/a', 'p/b'] }, { members: ['p/c', 'p/d', 'p/e'] }],
  blocks: [
    block('p/a', { complexity: { functions: 10, complex: 3 }, weight_in: 16, weight_out: 7 }),
    block('p/b', { weight_in: 3, weight_out: 9 }),
    block('p/c', { weight_out: 9 }),
  ],
  bands: [
    { source: 'p/c', target: 'p/a', weight: 9, kinds: {}, direction: 'up' },
    { source: 'p/d', target: 'p/b', weight: 2, kinds: {}, direction: 'up' },
    { source: 'p/a', target: 'p/c', weight: 4, kinds: {}, direction: 'skip' },
    { source: 'p/b', target: 'p/a', weight: 7, kinds: {}, direction: 'down' },
    { source: 'p/a', target: 'p/b', weight: 3, kinds: {}, direction: 'level' },
  ],
  budget: { drawn: 3, modules: 5, bands: 4, edges: 22 },
  libraries: { count: 0, top: [] },
};

// Each diagram is a facet (user, 2026-11-02): it says what to act on for what it
// draws, and nothing else — no shared cycles-then-upward lead on every lens.
describe('takeaways', () => {
  it('Structure: the layering — upward dependencies heaviest first, then layer skips; no cycles', () => {
    const t = takeaways('structure', L0, null, null);
    expect(t.map((x) => x.title)).toEqual(['2 upward dependencies', '1 layer skip']);
    expect(t[0].items.map((i) => [i.text, i.value, i.target])).toEqual([
      ['p/c → p/a', '9', 'p/c'],
      ['p/d → p/b', '2', 'p/d'],
    ]);
  });

  it('Cycles: each cycle, with the lightest dependency inside it as the one to cut', () => {
    const t = takeaways('cycles', L0, null, null);
    expect(t.map((x) => x.title)).toEqual(['Break 2 cycles']);
    expect(t[0].items.map((i) => [i.text, i.value, i.target])).toEqual([
      ['p/a ⇄ p/b', 'cut p/a → p/b (3)', 'p/a'],
      ['p/c ⇄ p/d ⇄ p/e', '', 'p/c'],
    ]);
  });

  it('Matrix: the cells to clear — mutual dependencies, then upward ones', () => {
    const t = takeaways('matrix', L0, null, null);
    // p/a and p/c depend on each other (4 one way, 9 the other) as well as p/a and p/b;
    // the heavier pair comes first.
    expect(t.map((x) => x.title)).toEqual(['2 mutual dependencies', '2 upward dependencies']);
    expect(t[0].items.map((i) => [i.text, i.value])).toEqual([['p/a ⇄ p/c', '4 · 9'], ['p/a ⇄ p/b', '3 · 7']]);
  });

  it('Zones: only the zone of pain and the zone of uselessness', () => {
    const z: Zones = {
      meanDistance: 0.3, coverage: { points: 3, unplaceable: 0 },
      points: [
        { component: 'p/config', types: 2, abstractTypes: 0, ca: 19, ce: 5, abstractness: 0, instability: 0.21, distance: 0.79, zone: 'pain' },
        { component: 'p/model', types: 5, abstractTypes: 0, ca: 7, ce: 3, abstractness: 0, instability: 0.3, distance: 0.7, zone: 'pain' },
        { component: 'p/ports', types: 4, abstractTypes: 4, ca: 0, ce: 2, abstractness: 1, instability: 1, distance: 1, zone: 'useless' },
        { component: 'p/fine', types: 4, abstractTypes: 2, ca: 3, ce: 3, abstractness: 0.5, instability: 0.5, distance: 0, zone: 'main' },
      ],
    };
    const t = takeaways('zones', L0, null, z);
    expect(t.map((x) => x.title)).toEqual(['2 modules in the zone of pain', '1 abstraction nothing uses']);
    expect(t[0].items.map((i) => [i.text, i.value])).toEqual([['p/config', '19 dependents'], ['p/model', '7 dependents']]);
  });

  it('Complexity: only the modules over 20%; Hidden coupling: only the strongest pairs', () => {
    expect(takeaways('complexity', L0, null, null)).toMatchObject([{ title: '1 module over 20% complex', items: [{ text: 'p/a', value: '30%' }] }]);
    const cc: CoChange = { hidden: [{ a: 'p/a', b: 'p/c', together: 7, strength: 0.7 }], pairs: 1, commits: 40, minTogether: 5 };
    expect(takeaways('coupling', L0, cc, null)).toMatchObject([{ title: '1 hidden coupling', items: [{ text: 'p/a ⇢ p/c', value: '7 commits' }] }]);
  });

  it('Wheel: the most depended-on modules, and the widest reach', () => {
    const t = takeaways('wheel', L0, null, null);
    expect(t.map((x) => x.title)).toEqual(['Most depended on', 'Widest reach']);
    expect(t[0].items.map((i) => [i.text, i.value])).toEqual([['p/a', '16'], ['p/b', '3']]);
    expect(t[1].items.map((i) => i.text)).toEqual(['p/b', 'p/c', 'p/a']);
  });

  it('no two lenses say the same thing first', () => {
    const z: Zones = { meanDistance: 0, coverage: { points: 0, unplaceable: 0 }, points: [] };
    const cc: CoChange = { hidden: [{ a: 'p/a', b: 'p/c', together: 7, strength: 0.7 }], pairs: 1, commits: 40, minTogether: 5 };
    const firsts = (['structure', 'cycles', 'matrix', 'coupling', 'complexity', 'wheel'] as const)
      .map((lens) => takeaways(lens, L0, cc, z)[0]?.title);
    expect(new Set(firsts).size).toBe(firsts.length);
  });

  it('a clean map says there is nothing to fix rather than an empty panel', () => {
    const clean = { ...L0, cycles: [], bands: [] };
    expect(takeaways('structure', clean, null, null)).toEqual([]);
    expect(takeaways('cycles', clean, null, null)).toEqual([]);
  });
});

describe('takeawaysMarkdown', () => {
  it('copies what to act on as markdown: a heading per takeaway, why, then the modules', () => {
    const md = takeawaysMarkdown('Structure', takeaways('structure', L0, null, null));
    expect(md).toBe([
      '## What to act on · Structure',
      '',
      '### 2 upward dependencies',
      '',
      'A lower layer calling up is a layering violation. Put an interface below and depend on it, or move the code to the layer that uses it.',
      '',
      '- p/c → p/a — 9',
      '- p/d → p/b — 2',
      '',
      '### 1 layer skip',
      '',
      'Not a violation: a layer bypassed. Worth a look where it is heavy, as the skipped layer may be missing a piece.',
      '',
      '- p/a → p/c — 4',
      '',
    ].join('\n'));
  });
});
