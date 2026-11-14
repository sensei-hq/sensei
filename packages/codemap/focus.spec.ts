import { describe, expect, it } from 'vitest';
import { focusComplexity, focusCoupling, focusSets, focusZones } from './focus';
import type { CoChange, ComplexityNode, ComplexityTree, Level0, Zones } from './types';

const l0 = (focus: Level0['focus']): Level0 => ({
  level: 'module', groupBy: 'layers', blocks: [], bands: [], cycles: [], coverage: { unplaced: 0 },
  budget: { drawn: 0, modules: 0, bands: 0, edges: 0 }, libraries: { count: 0, top: [] }, focus,
});
const FOCUS = { kind: 'module' as const, id: 'p/a', members: ['p/a'], neighbours: ['p/b'] };

const node = (id: string, functions: number, complex: number, maxCyclo: number, files: number): ComplexityNode =>
  ({ id, label: id, kind: 'module', functions, complex, maxCyclo, files, children: [] });

// A focus narrows every lens to the focused modules (user, 2026-11-03); the
// lenses the daemon does not narrow are narrowed here, from the focus it named.
describe('focus', () => {
  it('has no sets when the map is not focused', () => {
    expect(focusSets(l0(null))).toBeNull();
    expect(focusSets(l0(undefined))).toBeNull();
  });

  it('Hidden coupling keeps only the pairs a focused module is in', () => {
    const data: CoChange = {
      hidden: [
        { a: 'p/a', b: 'p/x', together: 9, strength: 0.9 },
        { a: 'p/y', b: 'p/a', together: 6, strength: 0.6 },
        { a: 'p/b', b: 'p/x', together: 8, strength: 0.8 },
      ],
      pairs: 12, commits: 40, minTogether: 5,
    };
    const got = focusCoupling(data, focusSets(l0(FOCUS)));
    expect(got?.hidden.map((c) => `${c.a}~${c.b}`)).toEqual(['p/a~p/x', 'p/y~p/a']);
    expect([got?.commits, got?.minTogether]).toEqual([40, 5]);
    expect(focusCoupling(data, null)).toBe(data);
  });

  it('Complexity keeps the focused modules’ branches, totals them, and drops the workspace’s maintainability', () => {
    const tree: ComplexityTree = {
      total: { functions: 30, complex: 7, maxCyclo: 40, files: 9, tier: 'danger' },
      modules: [node('p/a', 10, 3, 24, 4), node('p/b', 15, 4, 40, 4), node('p/c', 5, 0, 6, 1)],
      maintainability: { value: 3, grade: 'B', label: '3.0', date: '2026-11-03' },
    };
    const got = focusComplexity(tree, focusSets(l0(FOCUS)));
    expect(got?.modules.map((m) => m.id)).toEqual(['p/a']);
    expect(got?.total).toEqual({ functions: 10, complex: 3, maxCyclo: 24, files: 4, tier: null });
    expect(got?.maintainability).toBeNull();
    expect(focusComplexity(tree, null)).toBe(tree);
  });

  it('Zones keeps the focused modules and their neighbours', () => {
    const p = (component: string) => ({ component, types: 1, abstractTypes: 0, ca: 1, ce: 1, abstractness: 0, instability: 0.5, distance: 0.5, zone: 'main' as const });
    const zones: Zones = { points: [p('p/a'), p('p/b'), p('p/z')], meanDistance: 0.5, coverage: { points: 3, unplaceable: 0 } };
    const got = focusZones(zones, focusSets(l0(FOCUS)));
    expect(got?.points.map((x) => x.component)).toEqual(['p/a', 'p/b']);
    expect(got?.coverage).toEqual({ points: 2, unplaceable: 0 });
    expect(focusZones(zones, null)).toBe(zones);
  });

  it('a lens with nothing read yet stays unread under a focus', () => {
    const sets = focusSets(l0(FOCUS));
    expect([focusCoupling(null, sets), focusComplexity(null, sets), focusZones(null, sets)]).toEqual([null, null, null]);
  });

  // Review r1 F3: a mean over no placed point is no reading, not 0.00 ("on the
  // main sequence"); coverage means what the daemon means — points = all, and
  // unplaceable = those with no distance.
  it('a focused Zones with nothing placed has no mean, and counts coverage as the daemon does', () => {
    const unplaced = (component: string) => ({ component, types: 0, abstractTypes: 0, ca: 0, ce: 0, abstractness: 0, instability: 0, distance: null, zone: null });
    const zones: Zones = { points: [unplaced('p/a'), unplaced('p/b')], meanDistance: null, coverage: { points: 2, unplaceable: 2 } };
    const got = focusZones(zones, focusSets(l0(FOCUS)));
    expect(got?.meanDistance).toBeNull();
    expect(got?.coverage).toEqual({ points: 2, unplaceable: 2 });
  });
});
