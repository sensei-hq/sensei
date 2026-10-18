// The receiving half of the graph push (#225; docs/spec/dojo/graph-sync.md).
//
// The write and its gate are the database's: dojo.file_graph_slices re-checks
// the caller's verdict, the branch, the commit order, and takes the tenant from
// the verdict — proved against a real Postgres in
// database/tests/dojo/graph_sync_access.sql. What is left here: resolving the
// repo_key to ONE of the caller's repositories, refusing by name what this side
// can, checking the body's shape, and sending the batch in one call.
import { describe, it, expect, beforeEach } from 'vitest';
import { fakeDojoDb, resetFakeIds, type FakeTable } from './fake-dojo-db';
import { ingestGraph, graphHashes } from './graph-ingest';
import { AdminError } from './admin-data';

const MINE = { repository_id: 'r-api', tenant_id: 't-acme', repo_key: 'github.com/acme/api', sync_enabled: true };

function tables(mine: Record<string, unknown>[] = [MINE]): Record<string, FakeTable> {
	return {
		'rpc:my_repositories': { rows: mine },
		'rpc:file_graph_slices': { rows: [{ files: 1, refused_older: 0, pruned: 0 }] },
		'rpc:graph_file_hashes': {
			rows: [
				{ path: 'src/a.ts', content_hash: 'ha' },
				{ path: 'src/b.ts', content_hash: 'hb' }
			]
		}
	};
}

const COMMIT = { branch: 'main', sha: 'c1', committed_at: '2026-10-01T10:00:00Z' };
const SLICE = { path: 'src/a.ts', content_hash: 'ha', nodes: [], edges: [] };
const body = (over: Record<string, unknown> = {}) => ({
	repo_key: 'github.com/acme/api',
	commit: COMMIT,
	slices: [SLICE],
	...over
});

async function refusal(p: Promise<unknown>): Promise<number> {
	const e = await p.then(
		() => null,
		(err) => err
	);
	expect(e).toBeInstanceOf(AdminError);
	return (e as AdminError).status;
}

beforeEach(() => resetFakeIds());

describe('ingestGraph', () => {
	it('files the batch under the caller\'s repository, in one call, and reports what the database did', async () => {
		const db = fakeDojoDb(tables());
		const out = await ingestGraph(db as never, body({ manifest: { 'src/a.ts': 'ha' } }));
		expect(out).toEqual({ files: 1, refused_older: 0, pruned: 0 });
		const calls = db.rpcCalls.filter((c) => c.fn === 'file_graph_slices');
		expect(calls).toHaveLength(1);
		expect(calls[0].args).toEqual({
			p_repository_id: 'r-api',
			p_commit: COMMIT,
			p_slices: [SLICE],
			p_manifest: { 'src/a.ts': 'ha' }
		});
	});

	it('sends no manifest when none came — a missing manifest must never prune', async () => {
		const db = fakeDojoDb(tables());
		await ingestGraph(db as never, body());
		expect(db.rpcCalls.find((c) => c.fn === 'file_graph_slices')?.args.p_manifest).toBeNull();
	});

	it('answers one 404 for "never registered" and "another tenant\'s" — no existence oracle', async () => {
		const db = fakeDojoDb(tables([]));
		expect(await refusal(ingestGraph(db as never, body()))).toBe(404);
		expect(db.rpcCalls.some((c) => c.fn === 'file_graph_slices')).toBe(false);
	});

	it('refuses a repository of the caller\'s that may not sync (403), and one registered twice (409)', async () => {
		const shut = fakeDojoDb(tables([{ ...MINE, sync_enabled: false }]));
		expect(await refusal(ingestGraph(shut as never, body()))).toBe(403);
		const twice = fakeDojoDb(tables([MINE, { ...MINE, repository_id: 'r-other', tenant_id: 't-two' }]));
		expect(await refusal(ingestGraph(twice as never, body()))).toBe(409);
	});

	it('refuses a body it cannot read (400) before asking the database anything', async () => {
		for (const bad of [
			{ ...body(), repo_key: 7 },
			{ ...body(), commit: { branch: 'main', sha: 'c1' } },
			{ ...body(), commit: { ...COMMIT, branch: '' } },
			{ ...body(), slices: 'all' },
			{ ...body(), manifest: ['src/a.ts'] },
			null
		]) {
			const db = fakeDojoDb(tables());
			expect(await refusal(ingestGraph(db as never, bad as never))).toBe(400);
			expect(db.rpcCalls).toHaveLength(0);
		}
	});

	it('keeps the database\'s refusal as the status it means (an untracked branch is 422)', async () => {
		const db = fakeDojoDb({
			...tables(),
			'rpc:file_graph_slices': {
				rows: [],
				error: { code: '22023', message: 'branch_not_tracked: repository r-api does not track branch x' }
			}
		});
		const e = await ingestGraph(db as never, body()).catch((err) => err);
		expect(e).toBeInstanceOf(AdminError);
		expect(e.status).toBe(422);
		expect(e.message).toContain('branch_not_tracked');
	});
});

describe('graphHashes', () => {
	it('returns the accepted hashes of one branch as a path → hash map', async () => {
		const db = fakeDojoDb(tables());
		expect(await graphHashes(db as never, 'github.com/acme/api', 'main')).toEqual({
			files: { 'src/a.ts': 'ha', 'src/b.ts': 'hb' }
		});
		expect(db.rpcCalls.find((c) => c.fn === 'graph_file_hashes')?.args).toEqual({
			p_repository_id: 'r-api',
			p_branch: 'main'
		});
	});

	it('reads every file of a branch past PostgREST\'s 1000-row cap, page by page', async () => {
		// An unpaged read is truncated without a word, and every file past the cap
		// would look unsynced and be pushed again on every cycle.
		const rows = Array.from({ length: 2500 }, (_, i) => ({ path: `f${i}.ts`, content_hash: `h${i}` }));
		const db = fakeDojoDb({ ...tables(), 'rpc:graph_file_hashes': { rows } });
		const out = await graphHashes(db as never, 'github.com/acme/api', 'main');
		expect(Object.keys(out.files)).toHaveLength(2500);
		expect(out.files['f2499.ts']).toBe('h2499');
	});

	it('refuses as the write does: unknown 404, not permitted 403, empty branch 400', async () => {
		expect(await refusal(graphHashes(fakeDojoDb(tables([])) as never, 'github.com/acme/api', 'main'))).toBe(404);
		expect(
			await refusal(
				graphHashes(fakeDojoDb(tables([{ ...MINE, sync_enabled: false }])) as never, 'github.com/acme/api', 'main')
			)
		).toBe(403);
		expect(await refusal(graphHashes(fakeDojoDb(tables()) as never, 'github.com/acme/api', ''))).toBe(400);
	});
});
