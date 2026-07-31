// Unit tests for the per-engagement kept/stripped tally (`engagement-artifact-counts.ts`):
// the pure grouping and the tenant-scoped `countEngagementArtifacts` DB wrapper
// (dedup, published→kept / archived→stripped mapping, fail-closed on error).
import { describe, it, expect } from 'vitest';
import { tallyByEngagement, countEngagementArtifacts } from './engagement-artifact-counts';
import { AdminError, type DojoClient } from './admin-data';

describe('tallyByEngagement', () => {
	it('maps published → lessonsKept and archived → stripped, per engagement', () => {
		const rows = [
			{ engagement_id: 'e1', status: 'published' },
			{ engagement_id: 'e1', status: 'published' },
			{ engagement_id: 'e1', status: 'archived' },
			{ engagement_id: 'e2', status: 'archived' }
		];
		const m = tallyByEngagement(rows);
		expect(m.get('e1')).toEqual({ lessonsKept: 2, stripped: 1 });
		expect(m.get('e2')).toEqual({ lessonsKept: 0, stripped: 1 });
	});
	it('ignores rows with a null engagement_id or a non-counted status', () => {
		const rows = [
			{ engagement_id: null, status: 'published' },
			{ engagement_id: 'e1', status: 'submitted' },
			{ engagement_id: 'e1', status: 'published' }
		];
		const m = tallyByEngagement(rows);
		expect(m.get('e1')).toEqual({ lessonsKept: 1, stripped: 0 });
		expect(m.size).toBe(1);
	});
	it('returns an empty map for no rows', () => {
		expect(tallyByEngagement([])).toEqual(new Map());
	});
});

// A stub whose `.from().select().eq().in().in()` terminal resolves the result,
// capturing the engagement-id + status filters.
function makeDb(result: { data?: unknown; error: unknown }) {
	const captured: { ins: unknown[] } = { ins: [] };
	const b: Record<string, unknown> = {};
	b.from = () => b;
	b.select = () => b;
	b.eq = () => b;
	b.in = (_col: string, vals: unknown) => {
		captured.ins.push(vals);
		// the query chains two .in()s (engagement_id, status); the LAST resolves.
		return captured.ins.length >= 2 ? Promise.resolve(result) : b;
	};
	return { db: b as unknown as DojoClient, captured };
}

describe('countEngagementArtifacts', () => {
	it('returns an empty map (no query) for no engagement ids', async () => {
		const { db, captured } = makeDb({ data: [], error: null });
		expect(await countEngagementArtifacts(db, 't1', [])).toEqual(new Map());
		expect(captured.ins).toEqual([]); // never queried
	});
	it('dedups engagement ids and filters to published/archived', async () => {
		const { db, captured } = makeDb({ data: [], error: null });
		await countEngagementArtifacts(db, 't1', ['e1', 'e1', 'e2', '']);
		expect(captured.ins[0]).toEqual(['e1', 'e2']); // deduped + blanks dropped
		expect(captured.ins[1]).toEqual(['published', 'archived']);
	});
	it('tallies the returned rows into per-engagement counts', async () => {
		const { db } = makeDb({
			data: [
				{ engagement_id: 'e1', status: 'published' },
				{ engagement_id: 'e1', status: 'archived' },
				{ engagement_id: 'e2', status: 'published' }
			],
			error: null
		});
		const m = await countEngagementArtifacts(db, 't1', ['e1', 'e2']);
		expect(m.get('e1')).toEqual({ lessonsKept: 1, stripped: 1 });
		expect(m.get('e2')).toEqual({ lessonsKept: 1, stripped: 0 });
	});
	it('treats null data as no artifacts', async () => {
		const { db } = makeDb({ data: null, error: null });
		expect(await countEngagementArtifacts(db, 't1', ['e1'])).toEqual(new Map());
	});
	it('throws AdminError(500) on a query error (fail-closed)', async () => {
		const { db } = makeDb({ data: null, error: { message: 'boom' } });
		await expect(countEngagementArtifacts(db, 't1', ['e1'])).rejects.toBeInstanceOf(AdminError);
	});
});
