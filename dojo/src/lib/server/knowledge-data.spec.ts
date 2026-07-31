// Unit tests for the maintainer Knowledge read (`knowledge-data.ts`): the pure
// partition (catalog vs active vs pending) + prune-window pick, and the
// tenant-scoped `getKnowledgeLibrary` composition (fail-closed on either query).
import { describe, it, expect } from 'vitest';
import {
	partitionKnowledge,
	tightestRetention,
	getKnowledgeLibrary,
	KnowledgeError,
	type KnowledgeArtifact,
	type DojoClient
} from './knowledge-data';

const NOW = new Date('2026-07-30T00:00:00Z');

function art(over: Partial<KnowledgeArtifact> = {}): KnowledgeArtifact {
	return {
		id: 'a1',
		kind: 'principle',
		title: 'Idempotency key on money mutations',
		scope: { team: 'Payments' },
		adopted_count: 3,
		created_at: '2026-07-01T00:00:00Z',
		...over
	};
}

describe('partitionKnowledge', () => {
	it('routes extension kinds (skill/agent/prompt) to catalog', () => {
		const rows = [art({ kind: 'skill' }), art({ kind: 'agent' }), art({ kind: 'prompt' })];
		const { catalog, active, pending } = partitionKnowledge(rows, 90, NOW);
		expect(catalog).toHaveLength(3);
		expect(active).toHaveLength(0);
		expect(pending).toHaveLength(0);
	});
	it('keeps used knowledge active', () => {
		const { active, pending } = partitionKnowledge([art({ adopted_count: 5 })], 90, NOW);
		expect(active).toHaveLength(1);
		expect(pending).toHaveLength(0);
	});
	it('marks unused knowledge older than the window as pending', () => {
		// created 2026-01-01 → ~210d old > 90d window, adopted_count 0
		const stale = art({ adopted_count: 0, created_at: '2026-01-01T00:00:00Z' });
		const { active, pending } = partitionKnowledge([stale], 90, NOW);
		expect(pending).toHaveLength(1);
		expect(active).toHaveLength(0);
	});
	it('keeps a recent unused artifact active (within the window)', () => {
		const recent = art({ adopted_count: 0, created_at: '2026-07-20T00:00:00Z' }); // 10d < 90
		expect(partitionKnowledge([recent], 90, NOW).active).toHaveLength(1);
	});
	it('never prunes when no retention window is set', () => {
		const stale = art({ adopted_count: 0, created_at: '2020-01-01T00:00:00Z' });
		const { active, pending } = partitionKnowledge([stale], null, NOW);
		expect(active).toHaveLength(1);
		expect(pending).toHaveLength(0);
	});
});

describe('tightestRetention', () => {
	it('picks the smallest non-null retention_days', () => {
		expect(tightestRetention([{ retention_days: 180 }, { retention_days: 30 }, { retention_days: null }])).toBe(30);
	});
	it('is null when every policy is null / there are none', () => {
		expect(tightestRetention([{ retention_days: null }])).toBeNull();
		expect(tightestRetention([])).toBeNull();
	});
});

// A table-aware stub: each `.from(table)` returns its own terminal (artifacts ends
// on `.order()`, policies on `.eq()` awaited via `.then`).
function makeDb(tables: Record<string, { data: unknown; error: unknown }>) {
	return {
		from(table: string) {
			const res = tables[table] ?? { data: null, error: null };
			const b: Record<string, unknown> = {};
			b.select = () => b;
			b.eq = () => b;
			b.order = () => Promise.resolve(res);
			b.then = (resolve: (v: unknown) => unknown) => resolve(res);
			return b;
		}
	} as unknown as DojoClient;
}

describe('getKnowledgeLibrary', () => {
	it('composes the partitioned library + prune window', async () => {
		const db = makeDb({
			artifacts: {
				data: [
					art({ id: 'p1', kind: 'principle', adopted_count: 4 }),
					art({ id: 's1', kind: 'skill' }),
					art({ id: 'stale', kind: 'guard', adopted_count: 0, created_at: '2026-01-01T00:00:00Z' })
				],
				error: null
			},
			policies: { data: [{ retention_days: 90 }, { retention_days: 30 }], error: null }
		});
		const lib = await getKnowledgeLibrary(db, 't1', NOW);
		expect(lib.retention_days).toBe(30);
		expect(lib.active.map((a) => a.id)).toEqual(['p1']);
		expect(lib.catalog.map((a) => a.id)).toEqual(['s1']);
		expect(lib.pending.map((a) => a.id)).toEqual(['stale']);
	});
	it('is honest-empty when the tenant has no artifacts', async () => {
		const db = makeDb({ artifacts: { data: [], error: null }, policies: { data: [], error: null } });
		const lib = await getKnowledgeLibrary(db, 't1', NOW);
		expect(lib).toEqual({ retention_days: null, active: [], pending: [], catalog: [] });
	});
	it('fails closed (500) on the artifacts query error — never a fixture', async () => {
		const db = makeDb({ artifacts: { data: null, error: { message: 'boom' } }, policies: { data: [], error: null } });
		await expect(getKnowledgeLibrary(db, 't1', NOW)).rejects.toBeInstanceOf(KnowledgeError);
	});
	it('fails closed (500) on the policies query error', async () => {
		const db = makeDb({ artifacts: { data: [], error: null }, policies: { data: null, error: { message: 'boom' } } });
		await expect(getKnowledgeLibrary(db, 't1', NOW)).rejects.toBeInstanceOf(KnowledgeError);
	});
});
