// Unit tests for the dōjō graph reads (`graph-reads.ts`, #268): one branch's
// files/nodes/edges, read-time (folder, fqn) resolution with the unresolved
// kept as externals, and the branch list with its tracked default.
// Fail-closed on error; an unsynced branch is an honest empty.
import { describe, it, expect } from 'vitest';
import { readBranchGraph, listBranches, AdminError, type DojoClient } from './graph-reads';

// Table-keyed fake: each `.from(table)` chain resolves that table's canned rows
// at `.range()` (paged reads) or `.order()` (small reads), recording every
// narrowing call so tests pin the (repository_id, branch) scoping.
function makeDb(tables: Record<string, { data?: unknown[]; error?: { message: string } | null }>) {
	let table = '';
	const calls: Array<{ table: string; op: string; args: unknown[] }> = [];
	const b: Record<string, unknown> = {};
	b.from = (t: string) => ((table = t), b);
	b.select = () => b;
	b.eq = (...args: unknown[]) => {
		calls.push({ table, op: 'eq', args });
		return b;
	};
	b.order = (...args: unknown[]) => {
		calls.push({ table, op: 'order', args });
		return b;
	};
	b.range = (from: number, to: number) => {
		calls.push({ table, op: 'range', args: [from, to] });
		const t = tables[table] ?? { data: [] };
		const rows = (t.data ?? []).slice(from, to + 1);
		return Promise.resolve({ data: rows, error: t.error ?? null });
	};
	b.rpc = (fn: string) => {
		calls.push({ table: fn, op: 'rpc', args: [] });
		// Chainable like the table reads: select/order narrow, range pages.
		const r: Record<string, unknown> = {};
		r.select = () => r;
		r.order = (...args: unknown[]) => {
			calls.push({ table: fn, op: 'order', args });
			return r;
		};
		r.range = (from: number, to: number) => {
			const t = tables[`rpc:${fn}`] ?? { data: [] };
			return Promise.resolve({ data: (t.data ?? []).slice(from, to + 1), error: t.error ?? null });
		};
		return r;
	};
	return { db: b as unknown as DojoClient, calls };
}

const FILES = [{ path: 'src/a.ts', content_hash: 'h1', commit_sha: 'c1', committed_at: '2026-11-01T00:00:00Z' }];
const NODES = [
	{ id: 'n1', file_path: 'src/a.ts', folder_path: 'src', fqn: 'a', kind: 'function', name: 'a', parent_fqn: null, line_start: 1, line_end: 9, is_exported: true, is_test: false, signature: 'a()', language: 'typescript', community_id: null, tags: [] },
	{ id: 'n2', file_path: 'src/a.ts', folder_path: 'src', fqn: 'b', kind: 'function', name: 'b', parent_fqn: null, line_start: 11, line_end: 20, is_exported: false, is_test: false, signature: 'b()', language: 'typescript', community_id: null, tags: [] }
];

describe('readBranchGraph', () => {
	it('scopes files, nodes and edges to the repository and branch, ordered deterministically', async () => {
		const { db, calls } = makeDb({ graph_files: { data: FILES }, graph_nodes: { data: NODES }, graph_edges: { data: [] } });
		const out = await readBranchGraph(db, 'r1', 'main');
		expect(out.files).toHaveLength(1);
		expect(out.nodes).toHaveLength(2);
		for (const table of ['graph_files', 'graph_nodes', 'graph_edges']) {
			const eqs = calls.filter((c) => c.table === table && c.op === 'eq').map((c) => c.args);
			expect(eqs).toContainEqual(['repository_id', 'r1']);
			expect(eqs).toContainEqual(['branch', 'main']);
			expect(calls.some((c) => c.table === table && c.op === 'order')).toBe(true);
		}
	});

	it('resolves an edge target against the same branch nodes', async () => {
		const { db } = makeDb({
			graph_files: { data: FILES },
			graph_nodes: { data: NODES },
			graph_edges: {
				data: [{ id: 7, source_id: 'n1', kind: 'calls', target_folder: 'src', target_fqn: 'b', target_name: 'b', confidence: 'high', resolved_via: null, unresolved_reason: null }]
			}
		});
		const out = await readBranchGraph(db, 'r1', 'main');
		expect(out.edges).toEqual([
			expect.objectContaining({ id: 7, source_id: 'n1', target_id: 'n2', external: false })
		]);
	});

	it('keeps a dangling target key as an external, never dropped', async () => {
		const { db } = makeDb({
			graph_files: { data: FILES },
			graph_nodes: { data: NODES },
			graph_edges: {
				data: [{ id: 8, source_id: 'n1', kind: 'calls', target_folder: 'lib', target_fqn: 'missing', target_name: 'missing', confidence: null, resolved_via: null, unresolved_reason: 'unresolved' }]
			}
		});
		const out = await readBranchGraph(db, 'r1', 'main');
		expect(out.edges).toEqual([
			expect.objectContaining({ id: 8, target_id: null, external: true, target_folder: 'lib', target_fqn: 'missing' })
		]);
	});

	it('keeps a keyless edge (name only) as an external', async () => {
		const { db } = makeDb({
			graph_files: { data: FILES },
			graph_nodes: { data: NODES },
			graph_edges: {
				data: [{ id: 9, source_id: 'n1', kind: 'calls', target_folder: null, target_fqn: null, target_name: 'console.log', confidence: null, resolved_via: null, unresolved_reason: 'external' }]
			}
		});
		const out = await readBranchGraph(db, 'r1', 'main');
		expect(out.edges).toEqual([expect.objectContaining({ id: 9, target_id: null, external: true })]);
	});

	it('an unsynced branch is an honest empty, not an error', async () => {
		const { db } = makeDb({});
		expect(await readBranchGraph(db, 'r1', 'dev')).toEqual({ files: [], nodes: [], edges: [] });
	});

	it('a query error throws, never []-as-success', async () => {
		const { db } = makeDb({ graph_files: { data: [], error: { message: 'boom' } } });
		await expect(readBranchGraph(db, 'r1', 'main')).rejects.toThrow(AdminError);
	});
});

describe('listBranches', () => {
	it('lists tracked branches with the flagged default', async () => {
		const { db } = makeDb({
			'rpc:my_graph_branches': { data: [{ repository_id: 'r1', branch: 'main', is_default: true }, { repository_id: 'r1', branch: 'dev', is_default: false }] }
		});
		expect(await listBranches(db, 'r1')).toEqual({
			branches: [
				{ branch: 'main', is_default: true },
				{ branch: 'dev', is_default: false }
			],
			def: 'main'
		});
	});

	it('no flagged default is null, not a guess', async () => {
		const { db } = makeDb({ 'rpc:my_graph_branches': { data: [{ repository_id: 'r1', branch: 'dev', is_default: false }] } });
		expect(await listBranches(db, 'r1')).toEqual({ branches: [{ branch: 'dev', is_default: false }], def: null });
	});
});
