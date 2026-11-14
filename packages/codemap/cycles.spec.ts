import { describe, expect, it } from 'vitest';
import { cycleRings } from './cycles';
import type { Band, Level0 } from './types';

const band = (source: string, target: string, weight: number): Band => ({ source, target, weight, kinds: {}, direction: 'level' });
const L0 = {
  level: 'module', groupBy: 'layers', coverage: { unplaced: 0 }, blocks: [],
  cycles: [{ members: ['p/a', 'p/b'] }, { members: ['p/c', 'p/d', 'p/e'] }],
  bands: [band('p/a', 'p/b', 3), band('p/b', 'p/a', 7), band('p/c', 'p/d', 5), band('p/d', 'p/e', 2), band('p/e', 'p/c', 9), band('p/a', 'p/c', 4)],
  budget: { drawn: 0, modules: 0, bands: 0, edges: 0 }, libraries: { count: 0, top: [] },
} as unknown as Level0;

// The Cycles lens draws the cycles and nothing else (user, 2026-11-02): one ring per
// cycle, its members round it, the dependencies that close it, the lightest marked
// as the one to cut.
describe('cycleRings', () => {
  const rings = cycleRings(L0);

  it('is one ring per cycle, its members placed round it', () => {
    expect(rings.map((r) => r.nodes.map((n) => n.id))).toEqual([['p/a', 'p/b'], ['p/c', 'p/d', 'p/e']]);
    for (const r of rings) for (const n of r.nodes) {
      expect(Math.hypot(n.x - r.size / 2, n.y - r.size / 2)).toBeCloseTo(r.radius, 5);
    }
  });

  it('draws only the dependencies inside the cycle, and marks the lightest to cut', () => {
    expect(rings[1].edges.map((e) => [e.source, e.target, e.weight, e.cut])).toEqual([
      ['p/c', 'p/d', 5, false],
      ['p/d', 'p/e', 2, true],
      ['p/e', 'p/c', 9, false],
    ]);
    expect(rings[0].edges.filter((e) => e.cut).map((e) => `${e.source}>${e.target}`)).toEqual(['p/a>p/b']);
  });

  it('bends a pair that depends both ways apart, so the two arrows do not overlap', () => {
    const [ab, ba] = rings[0].edges;
    expect(ab.path).not.toBe(ba.path);
    expect(ab.path.includes('Q')).toBe(true);
  });

  it('is empty for a map with no cycles', () => {
    expect(cycleRings({ ...L0, cycles: [] } as Level0)).toEqual([]);
  });
});
