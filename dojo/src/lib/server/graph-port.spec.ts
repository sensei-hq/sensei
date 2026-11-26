// Unit tests for the dōjō's CodeMapPort (`graph-port.ts`, #268): the exact
// `@sensei/codemap` wire shapes over one branch's graph rows — L0
// (structure/layers), module, file, zones — with cochange honestly empty
// (no history is synced) and complexity an honest error (Measures are not
// synced). Unknown modules/files are ok:false, never empty stand-ins.
import { describe, it, expect } from 'vitest';
import { createGraphPort } from './graph-port';
import type { BranchGraph } from './graph-reads';
import type { Level0, Level1, Level2, Zones } from '@sensei/codemap';

const node = (id: string, file: string | null, folder: string, fqn: string, kind = 'function', extra = {}) => ({
	id,
	file_path: file,
	folder_path: folder,
	fqn,
	kind,
	name: fqn.split('.').pop() as string,
	parent_fqn: null,
	line_start: 1,
	line_end: 9,
	is_exported: false,
	is_test: false,
	signature: null,
	language: 'typescript',
	community_id: null,
	tags: [] as string[],
	...extra
});

// shop/ calls web/; web/ calls back (a cycle); lib/ is imported from outside.
function graph(): BranchGraph {
	return {
		files: [
			{ path: 'shop/cart.ts', content_hash: 'h1', commit_sha: 'c', committed_at: '2026-11-01T00:00:00Z' },
			{ path: 'web/app.ts', content_hash: 'h2', commit_sha: 'c', committed_at: '2026-11-01T00:00:00Z' }
		],
		nodes: [
			node('n1', 'shop/cart.ts', 'shop', 'Cart', 'class'),
			node('n2', 'shop/cart.ts', 'shop', 'Cart.add', 'function', { line_start: 5 }),
			node('n3', 'web/app.ts', 'web', 'serve', 'function'),
			node('n4', null, '', 'Orphan', 'function')
		],
		edges: [
			{ id: 1, source_id: 'n2', kind: 'calls', target_folder: 'web', target_fqn: 'serve', target_name: 'serve', confidence: 'high', resolved_via: null, unresolved_reason: null, target_id: 'n3', external: false },
			{ id: 2, source_id: 'n3', kind: 'calls', target_folder: 'shop', target_fqn: 'Cart', target_name: 'Cart', confidence: 'high', resolved_via: null, unresolved_reason: null, target_id: 'n1', external: false },
			{ id: 3, source_id: 'n1', kind: 'imports', target_folder: null, target_fqn: null, target_name: 'react', confidence: null, resolved_via: null, unresolved_reason: 'external', target_id: null, external: true }
		]
	};
}

describe('level0', () => {
	it('folders group: one block per folder, bands per resolved pair, verdicts from the layering', async () => {
		const port = createGraphPort(async () => graph());
		const read = await port.level0('folders', null);
		expect(read.ok).toBe(true);
		if (!read.ok) return;
		const l0 = read.data as Level0;
		expect(l0.level).toBe('module');
		expect(l0.blocks.map((b) => b.id).sort()).toEqual(['shop', 'web']);
		expect(l0.blocks.find((b) => b.id === 'shop')).toMatchObject({ files: 1, symbols: 2 });
		expect(l0.bands).toHaveLength(2);
		// shop ↔ web is a cycle: exactly one band climbs.
		expect(l0.bands.filter((b) => b.direction === 'up')).toHaveLength(1);
		expect(l0.cycles).toHaveLength(1);
		expect(l0.coverage).toEqual({ unplaced: 1 });
		expect(l0.libraries).toEqual({ count: 1, top: ['react'] });
	});

	it('layers group: members share the cycle rank under L-labels', async () => {
		const port = createGraphPort(async () => graph());
		const read = await port.level0('layers', null);
		expect(read.ok).toBe(true);
		if (!read.ok) return;
		const groups = new Set((read.data as Level0).blocks.map((b) => b.group));
		expect(groups.size).toBe(1);
		expect([...groups][0]).toMatch(/^L\d+$/);
	});

	it('focus narrows to the module and its neighbours and says so', async () => {
		const port = createGraphPort(async () => graph());
		const read = await port.level0('folders', { kind: 'module', id: 'shop' });
		expect(read.ok).toBe(true);
		if (!read.ok) return;
		const l0 = read.data as Level0;
		expect(l0.blocks.map((b) => b.id).sort()).toEqual(['shop', 'web']);
		expect(l0.focus).toMatchObject({ kind: 'module', id: 'shop', members: ['shop'], neighbours: ['web'] });
	});

	it('a focus naming nothing here is ok:false', async () => {
		const port = createGraphPort(async () => graph());
		expect(await port.level0('folders', { kind: 'module', id: 'nope' })).toEqual({ ok: false, error: expect.any(String) });
	});

	it('an unsynced branch is an honest empty', async () => {
		const port = createGraphPort(async () => ({ files: [], nodes: [], edges: [] }));
		const read = await port.level0('folders', null);
		expect(read.ok).toBe(true);
		if (!read.ok) return;
		expect((read.data as Level0).blocks).toEqual([]);
		expect((read.data as Level0).budget).toEqual({ drawn: 0, modules: 0, bands: 0, edges: 0 });
	});
});

describe('module', () => {
	it('files in role columns with weighted, directed neighbours', async () => {
		const port = createGraphPort(async () => graph());
		const read = await port.module('shop');
		expect(read.ok).toBe(true);
		if (!read.ok) return;
		const l1 = read.data as Level1;
		expect(l1.module).toBe('shop');
		expect(l1.columns.map((c) => c.role)).toEqual(['code']);
		expect(l1.columns[0].files.map((f) => f.path)).toEqual(['shop/cart.ts']);
		expect(l1.dependsOn).toEqual([{ module: 'web', weight: 1, direction: expect.any(String) }]);
		expect(l1.usedBy).toEqual([{ module: 'web', weight: 1, direction: expect.any(String) }]);
	});

	it('an unknown module is ok:false', async () => {
		const port = createGraphPort(async () => graph());
		expect(await port.module('nope')).toEqual({ ok: false, error: expect.any(String) });
	});
});

describe('file', () => {
	it('symbols in source order with callers and callees, heaviest first', async () => {
		const port = createGraphPort(async () => graph());
		const read = await port.file('shop/cart.ts');
		expect(read.ok).toBe(true);
		if (!read.ok) return;
		const l2 = read.data as Level2;
		expect(l2.path).toBe('shop/cart.ts');
		expect(l2.role).toBe('code');
		expect(l2.symbols.map((s) => s.name)).toEqual(['Cart', 'add']);
		expect(l2.callers).toEqual([{ id: 'n3', name: 'serve', file: 'web/app.ts', weight: 1 }]);
		expect(l2.callersTotal).toBe(1);
		// Placed calls only, like the daemon: the external react import and
		// same-file edges are not neighbours.
		expect(l2.callees).toEqual([{ id: 'n3', name: 'serve', file: 'web/app.ts', weight: 1 }]);
		expect(l2.calleesTotal).toBe(1);
	});

	it('an unknown file is ok:false', async () => {
		const port = createGraphPort(async () => graph());
		expect(await port.file('nope.ts')).toEqual({ ok: false, error: expect.any(String) });
	});
});

describe('zones', () => {
	it('points per module with abstractness from declarations', async () => {
		const port = createGraphPort(async () => graph());
		const read = await port.zones();
		expect(read.ok).toBe(true);
		if (!read.ok) return;
		const z = read.data as Zones;
		expect(z.points.map((p) => p.component).sort()).toEqual(['shop', 'web']);
		const shop = z.points.find((p) => p.component === 'shop')!;
		expect(shop.types).toBe(1);
		expect(shop.ca).toBe(1);
		expect(shop.ce).toBe(2);
		// web declares no type: null abstractness, omitted from the plot, counted.
		expect(z.coverage).toEqual({ points: 2, unplaceable: 1 });
		expect(typeof z.meanDistance === 'number' || z.meanDistance === null).toBe(true);
	});
});

describe('cochange and complexity', () => {
	it('cochange answers no history, honestly', async () => {
		const port = createGraphPort(async () => graph());
		expect(await port.cochange()).toEqual({ ok: true, data: { hidden: [], pairs: 0, commits: 0, minTogether: 5 } });
	});

	it('complexity says Measures are not synced', async () => {
		const port = createGraphPort(async () => graph());
		const read = await port.complexity();
		expect(read.ok).toBe(false);
		if (read.ok) return;
		expect(read.error).toMatch(/not synced/);
	});
});
