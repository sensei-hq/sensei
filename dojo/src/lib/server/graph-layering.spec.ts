// Unit tests for the dōjō's layering (`graph-layering.ts`, #268) — a faithful
// port of the daemon's `analysis::layering::analyse`, which the dōjō cannot
// call: ranks over the condensation, one feedback order per cycle, a verdict
// per edge. Cases mirror the Rust suite; any intentional divergence is noted.
import { describe, it, expect } from 'vitest';
import { analyse, type Dep, type Layering, type Conformance } from './graph-layering';

const dep = (source: string, target: string, occurrences: number): Dep => ({ source, target, occurrences });
const layerOf = (l: Layering, id: string): number => l.units.find((u) => u.id === id)?.layer ?? -1;
const verdict = (l: Layering, s: string, t: string): Conformance =>
	l.deps.find((d) => d.source === s && d.target === t)?.conformance ?? 'level';

describe('analyse', () => {
	it('a caller ranks above what it calls, 0 at the top', () => {
		const l = analyse(['a', 'b'], [dep('a', 'b', 1)]);
		expect(layerOf(l, 'a')).toBe(0);
		expect(layerOf(l, 'b')).toBe(1);
		expect(verdict(l, 'a', 'b')).toBe('down');
		expect(l.depth).toBe(2);
	});

	it('a unit in no dependency is still placed', () => {
		const l = analyse(['a', 'b', 'lonely'], [dep('a', 'b', 1)]);
		expect(l.units).toHaveLength(3);
		expect(layerOf(l, 'lonely')).toBe(0);
	});

	it('a mutual pair collapses into one component sharing a layer, closing with exactly one edge', () => {
		const l = analyse(['a', 'b'], [dep('a', 'b', 2), dep('b', 'a', 3)]);
		const [a, b] = l.units;
		expect(a.component).toBe(b.component);
		expect(a.component_size).toBe(2);
		expect(a.layer).toBe(b.layer);
		expect(l.deps.filter((d) => d.conformance === 'up')).toHaveLength(1);
		expect(l.cycles).toHaveLength(1);
		expect(l.cycles[0].members).toEqual(['a', 'b']);
	});

	it('a cycle names the dependency cheapest to cut, inner heaviest first', () => {
		const l = analyse(['a', 'b', 'c'], [dep('a', 'b', 9), dep('b', 'c', 3), dep('c', 'a', 5)]);
		expect(l.cycles).toHaveLength(1);
		expect(l.cycles[0].members).toEqual(['a', 'b', 'c']);
		expect(l.cycles[0].cut).toEqual(dep('b', 'c', 3));
		expect(l.cycles[0].inner[0].occurrences).toBe(9);
		expect(l.deps.filter((d) => d.weakest)).toHaveLength(1);
	});

	it('a dependency reaching past a layer is a skip', () => {
		const l = analyse(['a', 'b', 'c'], [dep('a', 'b', 1), dep('b', 'c', 1), dep('a', 'c', 1)]);
		expect([layerOf(l, 'a'), layerOf(l, 'b'), layerOf(l, 'c')]).toEqual([0, 1, 2]);
		expect(verdict(l, 'a', 'b')).toBe('down');
		expect(verdict(l, 'b', 'c')).toBe('down');
		expect(verdict(l, 'a', 'c')).toBe('skip');
	});

	it('a self-dependency is kept aside, not ranked', () => {
		const l = analyse(['a', 'b'], [dep('a', 'a', 4), dep('a', 'b', 1)]);
		expect(l.self_deps).toEqual([dep('a', 'a', 4)]);
		expect(l.deps.map((d) => [d.source, d.target])).toEqual([['a', 'b']]);
		expect(layerOf(l, 'a')).toBe(0);
	});

	it('a dependency naming an unknown unit is dropped AND counted', () => {
		const l = analyse(['a'], [dep('a', 'ghost', 2)]);
		expect(l.deps).toEqual([]);
		expect(l.dropped).toEqual([dep('a', 'ghost', 2)]);
	});

	it('parallel edges fold: occurrences sum for the cut heuristic', () => {
		const l = analyse(['a', 'b'], [dep('a', 'b', 2), dep('a', 'b', 3)]);
		expect(l.deps).toEqual([
			expect.objectContaining({ source: 'a', target: 'b', occurrences: 5, conformance: 'down' })
		]);
	});

	it('the same graph in another row order ranks identically', () => {
		const deps = [dep('a', 'b', 1), dep('b', 'c', 1), dep('c', 'a', 1), dep('a', 'c', 2)];
		const l1 = analyse(['c', 'a', 'b'], deps);
		const l2 = analyse(['a', 'b', 'c'], [...deps].reverse());
		expect(JSON.stringify(l1)).toBe(JSON.stringify(l2));
	});

	it('an empty universe has depth 0 and no rows', () => {
		expect(analyse([], [])).toEqual({ units: [], deps: [], cycles: [], self_deps: [], dropped: [], depth: 0 });
	});
});
