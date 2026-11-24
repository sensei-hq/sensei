// Unit tests for the dōjō's codemap math (`graph-codemap.ts`, #268) — a
// faithful port of the daemon's `analysis::codemap` L0/L1 pieces the port
// needs: roles from paths, role columns, block folding with hubs, focus
// scope, and Martin zones from graph rows. Cases mirror the Rust suite.
import { describe, it, expect } from 'vitest';
import {
	BLOCK_BUDGET,
	COLUMN_BUDGET,
	fileRole,
	foldId,
	focusScope,
	level0,
	level1,
	segmentMembers,
	zones,
	librariesSummary,
	type Module,
	type ModuleEdge,
	type ModuleFile
} from './graph-codemap';

const m = (id: string, group: string, symbols: number): Module => ({
	id,
	label: id,
	group,
	pkg: 'pkg',
	files: 1,
	symbols,
	roles: { code: 1, test: 0, docs: 0, config: 0, generated: 0 },
	complexity: null
});
const e = (source: string, target: string, kind: string, count: number): ModuleEdge => ({ source, target, kind, count });
const band = (l0: ReturnType<typeof level0>, s: string, t: string) => l0.bands.find((b) => b.source === s && b.target === t);

describe('fileRole', () => {
	it.each([
		['src/lib.rs', 'code'],
		['app/src/lib/api.ts', 'code'],
		['crates/x/tests/routes.rs', 'test'],
		['src/lib/api.spec.ts', 'test'],
		['pkg/foo_test.go', 'test'],
		['tests/test_models.py', 'test'],
		['src/test/java/a/FooTest.java', 'test'],
		['e2e/tests/login.spec.ts', 'test'],
		['docs/guide/intro.md', 'docs'],
		['README.md', 'docs'],
		['package.json', 'config'],
		['svelte.config.js', 'config'],
		['Cargo.toml', 'config'],
		['src/gen/schema.pb.go', 'generated'],
		['src/__generated__/graphql.ts', 'generated'],
		['src/api_pb2.py', 'generated'],
		['src/__generated__/thing.test.ts', 'generated']
	])('%s reads as %s', (path, role) => {
		expect(fileRole(path)).toBe(role);
	});
});

describe('level1', () => {
	it('files sit in role columns, largest first, the rest folded and counted', () => {
		const f = (path: string, symbols: number): ModuleFile => ({ id: path, path, symbols });
		const files: ModuleFile[] = Array.from({ length: 15 }, (_, i) => f(`src/f${i}.rs`, 100 - i));
		files.push(f('tests/a.rs', 5));
		files.push(f('README.md', 1));
		const cols = level1(files);
		expect(cols.map((c) => c.role)).toEqual(['code', 'test', 'docs']);
		expect(cols[0].files).toHaveLength(COLUMN_BUDGET);
		expect(cols[0].files[0].path).toBe('src/f0.rs');
		expect([cols[0].more, cols[0].more_symbols]).toEqual([3, 88 + 87 + 86]);
		expect(cols.reduce((n, c) => n + c.files.length + c.more, 0)).toBe(files.length);
	});
});

describe('level0', () => {
	it('edges between two modules are one band; a module\u2019s own edges are not', () => {
		const mods = Array.from({ length: 40 }, (_, i) => m(`m${i}`, 'g', 10));
		const edges = Array.from({ length: 40 }, (_, i) => e(`m${(i + 1) % 40}`, `m${i}`, 'calls', 3));
		edges.push(e('m1', 'm0', 'imports', 2));
		edges.push(e('m0', 'm0', 'calls', 9));
		const l0 = level0(mods, edges, BLOCK_BUDGET);
		expect(l0.blocks.every((b) => !b.hub)).toBe(true);
		expect(band(l0, 'm1', 'm0')).toMatchObject({ weight: 5, kinds: { calls: 3, imports: 2 } });
		expect(l0.bands.filter((b) => b.source === 'm1' && b.target === 'm0')).toHaveLength(1);
		expect(band(l0, 'm0', 'm0')).toBeUndefined();
		expect(l0.budget.edges).toBe(40 * 3 + 2);
		expect(l0.budget.bands).toBe(40);
	});

	it('a hub draws no bands and carries its weight', () => {
		const mods = [...Array.from({ length: 30 }, (_, i) => m(`m${i}`, 'g', 10)), m('log', 'infra', 5)];
		const edges = [
			...Array.from({ length: 30 }, (_, i) => e(`m${i}`, 'log', 'calls', 2)),
			...Array.from({ length: 29 }, (_, k) => e(`m${k + 1}`, `m${k}`, 'calls', 1))
		];
		const l0 = level0(mods, edges, BLOCK_BUDGET);
		const log = l0.blocks.find((b) => b.id === 'log')!;
		expect(log.hub).toBe(true);
		expect(log.weight_in).toBe(60);
		expect(l0.bands.every((b) => b.target !== 'log' && b.source !== 'log')).toBe(true);
		expect(l0.blocks.filter((b) => b.id !== 'log').every((b) => !b.hub)).toBe(true);
		expect(l0.budget.edges).toBe(60 + 29);
	});

	it('over budget the smallest fold per group and their bands follow', () => {
		const mods: Module[] = [];
		for (const g of ['a', 'b']) for (let i = 0; i < 40; i++) mods.push(m(`${g}${i}`, g, 1000 - i));
		const edges = [
			e('a0', 'b0', 'calls', 3),
			e('a39', 'b39', 'calls', 5),
			e('a39', 'a38', 'calls', 7),
			e('a1', 'b39', 'calls', 2)
		];
		const l0 = level0(mods, edges, BLOCK_BUDGET);
		expect(l0.blocks.length).toBeLessThanOrEqual(BLOCK_BUDGET);
		expect(l0.budget.drawn).toBe(l0.blocks.length);
		expect(l0.budget.modules).toBe(80);
		const [fa, fb] = [foldId('a'), foldId('b')];
		const folded = l0.blocks.reduce((n, b) => n + b.folded.length, 0);
		const kept = l0.blocks.filter((b) => b.folded.length === 0).length;
		expect(kept + folded).toBe(80);
		expect(l0.blocks.some((b) => b.id === 'a0') && l0.blocks.some((b) => b.id === fa)).toBe(true);
		expect(l0.blocks.some((b) => b.id === 'a39')).toBe(false);
		expect(band(l0, 'a0', 'b0')?.weight).toBe(3);
		expect(band(l0, fa, fb)?.weight).toBe(5);
		expect(band(l0, 'a1', fb)?.weight).toBe(2);
		expect(band(l0, fa, fa)).toBeUndefined();
	});

	it('a small workspace has no hub', () => {
		const l0 = level0([m('a', 'g', 1), m('b', 'g', 1), m('c', 'g', 1)], [e('a', 'c', 'calls', 5), e('b', 'c', 'calls', 5)], BLOCK_BUDGET);
		expect(l0.blocks.every((b) => !b.hub)).toBe(true);
		expect(l0.bands).toHaveLength(2);
	});

	it('under budget every module is a block', () => {
		const l0 = level0(
			Array.from({ length: 10 }, (_, i) => m(`m${i}`, 'g', 1)),
			[],
			BLOCK_BUDGET
		);
		expect(l0.blocks).toHaveLength(10);
		expect(l0.blocks.every((b) => b.folded.length === 0)).toBe(true);
	});
});

describe('focus', () => {
	const mods = [m('a', 'g', 1), m('b', 'g', 1), m('c', 'g', 1), m('far', 'g', 1)];
	const edges = [e('a', 'b', 'calls', 1), e('c', 'a', 'calls', 1), e('c', 'far', 'calls', 1)];

	it('keeps the module, its direct neighbours, and only touching edges', () => {
		const f = focusScope(mods, edges, new Set(['a']));
		expect(f.modules.map((x) => x.id).sort()).toEqual(['a', 'b', 'c']);
		expect(f.neighbours).toEqual(['b', 'c']);
		expect(f.edges).toHaveLength(2);
	});

	it('a segment focuses every module in it', () => {
		expect([...segmentMembers(mods, 'g')].sort()).toEqual(['a', 'b', 'c', 'far']);
		expect(segmentMembers(mods, 'nope').size).toBe(0);
	});
});

describe('zones', () => {
	it('scores Martin points from module edges, nulls where the denominator is empty', () => {
		const pts = zones(
			[
				{ component: 'shop', types: 10, abstract: 5 },
				{ component: 'web', types: 2, abstract: 0 },
				{ component: 'empty', types: 0, abstract: 0 },
				{ component: 'alone', types: 4, abstract: 0 }
			],
			[
				{ source: 'web', target: 'shop' },
				{ source: 'web', target: 'shop' },
				{ source: 'shop', target: 'lib-a' },
				{ source: 'shop', target: 'lib-b' }
			],
			new Set(['lib-a', 'lib-b'])
		);
		const shop = pts.find((p) => p.component === 'shop')!;
		// Ca counts distinct internal partners (web once); Ce counts shop→libs (2 externals).
		expect(shop.ca).toBe(1);
		expect(shop.ce).toBe(2);
		expect(shop.abstractness).toBeCloseTo(0.5);
		expect(shop.instability).toBeCloseTo(2 / 3);
		expect(shop.zone).toBe('main');
		const empty = pts.find((p) => p.component === 'empty')!;
		expect(empty.abstractness).toBeNull();
		expect(empty.instability).toBeNull();
		expect(empty.zone).toBeNull();
	});

	it('pain and useless read off the distance thresholds', () => {
		const pts = zones(
			[
				{ component: 'rigid', types: 10, abstract: 0 },
				{ component: 'ivory', types: 10, abstract: 10 },
				{ component: 'u1', types: 1, abstract: 0 },
				{ component: 'u2', types: 1, abstract: 0 }
			],
			[
				{ source: 'u1', target: 'rigid' },
				{ source: 'u2', target: 'rigid' },
				{ source: 'ivory', target: 'lib' }
			],
			new Set(['lib'])
		);
		// rigid: A=0, I=0 → D=1, sum<1 → pain. ivory: A=1, I=1 → D=1, sum>1 → useless.
		expect(pts.find((p) => p.component === 'rigid')?.zone).toBe('pain');
		expect(pts.find((p) => p.component === 'ivory')?.zone).toBe('useless');
	});
});

describe('librariesSummary', () => {
	it('counts distinct libraries and names the most used', () => {
		expect(
			librariesSummary(
				[
					{ name: 'react', uses: 9 },
					{ name: 'lodash', uses: 3 },
					{ name: 'zod', uses: 5 }
				],
				2
			)
		).toEqual({ count: 3, top: ['react', 'zod'] });
	});
});
