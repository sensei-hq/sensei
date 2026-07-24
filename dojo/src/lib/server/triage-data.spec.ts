// Unit tests for the maintainer-triage store logic (`triage-data.ts`) — the TS
// port of dojo-mind's triage read + decide. Exercises:
//   • rankTriageRows — strongest-first (confidence desc, then newest; null last).
//   • shapeTriageRows — the wire shape (embedded-artifact kind/title
//     normalization) + the rank applied.
//   • parseDecideBody — status validation + the approve/decline required-field
//     guards.
//   • listTriage — open-state filter + join select + ranked result; error → 500.
//   • decideTriage — the 404-when-no-open-row guard, the decisions insert shape,
//     the state flip per verdict, and the DecideResult (artifact_id only on
//     approve).
// A chainable supabase-js stub (no live DB), like the sibling route/store specs.
import { describe, it, expect, vi, beforeEach } from 'vitest';
import {
	rankTriageRows,
	shapeTriageRows,
	parseDecideBody,
	listTriage,
	decideTriage,
	TriageError,
	type TriageRow,
	type DojoClient
} from './triage-data';

function row(over: Partial<TriageRow> = {}): TriageRow {
	return {
		signature: 's',
		artifact_id: 'a',
		kind: 'pattern',
		title: 't',
		owner_scope: {},
		confidence: 0.5,
		contributor_count: 1,
		similarity: null,
		nearest_artifact_id: null,
		state: 'queued',
		created_at: '2026-07-01T00:00:00Z',
		...over
	};
}

describe('rankTriageRows', () => {
	it('orders by confidence desc, then newest, null-confidence last', () => {
		const ranked = rankTriageRows([
			row({ signature: 'lo', confidence: 0.3 }),
			row({ signature: 'null', confidence: null }),
			row({ signature: 'hi', confidence: 0.9 }),
			row({ signature: 'mid-old', confidence: 0.6, created_at: '2026-06-01T00:00:00Z' }),
			row({ signature: 'mid-new', confidence: 0.6, created_at: '2026-07-10T00:00:00Z' })
		]);
		expect(ranked.map((r) => r.signature)).toEqual(['hi', 'mid-new', 'mid-old', 'lo', 'null']);
	});
	it('is pure — does not mutate its input', () => {
		const input = [row({ confidence: 0.1 }), row({ confidence: 0.9 })];
		const before = input.map((r) => r.confidence);
		rankTriageRows(input);
		expect(input.map((r) => r.confidence)).toEqual(before);
	});
});

describe('shapeTriageRows', () => {
	it('lifts the embedded artifact kind/title onto the row (object embed)', () => {
		const [r] = shapeTriageRows([
			{
				signature: 's1',
				artifact_id: 'a1',
				owner_scope: { label: 'Payments' },
				confidence: 0.8,
				contributor_count: 3,
				similarity: 0.2,
				nearest_artifact_id: 'a2',
				state: 'queued',
				created_at: '2026-07-01T00:00:00Z',
				artifacts: { kind: 'guard', title: 'No secrets' }
			}
		]);
		expect(r.kind).toBe('guard');
		expect(r.title).toBe('No secrets');
		expect(r.contributor_count).toBe(3);
	});
	it('normalizes a one-element array embed and tolerates a null artifact', () => {
		const rows = shapeTriageRows([
			{ signature: 's1', artifact_id: 'a1', owner_scope: {}, confidence: 0.9, contributor_count: 1, similarity: null, nearest_artifact_id: null, state: 'queued', created_at: '2026-07-02T00:00:00Z', artifacts: [{ kind: 'skill', title: 'X' }] },
			{ signature: 's2', artifact_id: 'a2', owner_scope: {}, confidence: 0.5, contributor_count: 1, similarity: null, nearest_artifact_id: null, state: 'queued', created_at: '2026-07-01T00:00:00Z', artifacts: null }
		]);
		expect(rows[0].kind).toBe('skill');
		expect(rows[1].kind).toBe('');
		expect(rows[1].title).toBe('');
		// ranked strongest-first
		expect(rows.map((r) => r.signature)).toEqual(['s1', 's2']);
	});
});

describe('parseDecideBody', () => {
	it('rejects an unknown status', () => {
		expect(() => parseDecideBody({ status: 'bogus' })).toThrow(TriageError);
		expect(() => parseDecideBody({})).toThrow(TriageError);
	});
	it('requires distribution_scope to approve', () => {
		expect(() => parseDecideBody({ status: 'approve' })).toThrow(/distribution_scope/);
		expect(parseDecideBody({ status: 'approve', distribution_scope: ['Company'] }).status).toBe('approve');
	});
	it('requires a non-empty reason to decline', () => {
		expect(() => parseDecideBody({ status: 'decline' })).toThrow(/reason/);
		expect(() => parseDecideBody({ status: 'decline', reason: '  ' })).toThrow(/reason/);
		expect(parseDecideBody({ status: 'decline', reason: 'dup' }).reason).toBe('dup');
	});
	it('accepts revise with neither', () => {
		expect(parseDecideBody({ status: 'revise' }).status).toBe('revise');
	});
});

// ── a chainable supabase-js stub ─────────────────────────────────────────────
// Each terminal op (`.order()` / `.maybeSingle()` / `.insert()`, and an awaited
// `.update()` chain) consumes the next result in order. The builder is thenable
// so `await db.from(...).update(...).eq(...)` resolves to the next result.
type Terminal = { data: unknown; error: unknown };
function makeDb(results: Terminal[]) {
	const calls: { table?: string; op?: string; payload?: unknown; filters: [string, unknown][] }[] = [];
	let cur: (typeof calls)[number];
	let i = 0;
	const b: Record<string, unknown> = {};
	const settle = (): Terminal => results[i++] ?? { data: null, error: null };
	b.from = (t: string) => {
		cur = { table: t, filters: [] };
		calls.push(cur);
		return b;
	};
	b.select = () => b;
	b.insert = (p: unknown) => {
		cur.op = 'insert';
		cur.payload = p;
		return Promise.resolve(settle());
	};
	b.update = (p: unknown) => {
		cur.op = 'update';
		cur.payload = p;
		return b;
	};
	b.eq = (c: string, v: unknown) => {
		cur.filters.push([c, v]);
		return b;
	};
	b.in = () => b;
	b.order = () => Promise.resolve(settle());
	b.maybeSingle = () => Promise.resolve(settle());
	// Only an update chain is awaited directly (find uses .maybeSingle, list uses
	// .order); resolve it to the next result.
	b.then = (resolve: (v: Terminal) => unknown) =>
		resolve(cur?.op === 'update' ? settle() : { data: null, error: null });
	return { db: b as unknown as DojoClient, calls };
}

describe('listTriage', () => {
	it('joins the artifact and returns ranked rows', async () => {
		const { db } = makeDb([
			{
				data: [
					{ signature: 's-lo', artifact_id: 'a1', owner_scope: {}, confidence: 0.4, contributor_count: 1, similarity: null, nearest_artifact_id: null, state: 'queued', created_at: '2026-07-01T00:00:00Z', artifacts: { kind: 'pattern', title: 'Lo' } },
					{ signature: 's-hi', artifact_id: 'a2', owner_scope: {}, confidence: 0.95, contributor_count: 2, similarity: null, nearest_artifact_id: null, state: 'in_review', created_at: '2026-07-02T00:00:00Z', artifacts: { kind: 'guard', title: 'Hi' } }
				],
				error: null
			}
		]);
		const rows = await listTriage(db, 't1');
		expect(rows.map((r) => r.signature)).toEqual(['s-hi', 's-lo']);
		expect(rows[0].title).toBe('Hi');
	});
	it('throws TriageError(500) on a query error', async () => {
		const { db } = makeDb([{ data: null, error: { message: 'boom' } }]);
		await expect(listTriage(db, 't1')).rejects.toThrow(TriageError);
	});
});

describe('decideTriage', () => {
	beforeEach(() => vi.restoreAllMocks());

	it('404s when no open row matches the signature', async () => {
		const { db } = makeDb([{ data: null, error: null }]);
		await expect(
			decideTriage(db, 't1', 'sig', { status: 'revise' }, 'm1')
		).rejects.toMatchObject({ status: 404 });
	});

	it('approve → writes a decision, flips to resolved, returns approved + artifact_id', async () => {
		const { db, calls } = makeDb([
			{ data: { id: 'q1', artifact_id: 'a1', state: 'queued' }, error: null }, // find
			{ data: null, error: null }, // decisions insert
			{ data: null, error: null } // triage_queue update
		]);
		const res = await decideTriage(db, 't1', 'sig', { status: 'approve', distribution_scope: ['Company'] }, 'm1');
		expect(res).toEqual({ status: 'approved', artifact_id: 'a1' });
		const decision = calls.find((c) => c.table === 'decisions');
		expect(decision?.payload).toMatchObject({ tenant_id: 't1', artifact_id: 'a1', triage_id: 'q1', maintainer_id: 'm1', status: 'approve', automated: false });
		const upd = calls.find((c) => c.table === 'triage_queue' && c.op === 'update');
		expect(upd?.payload).toMatchObject({ state: 'resolved' });
	});

	it('decline → resolved, declined, no artifact_id', async () => {
		const { db } = makeDb([
			{ data: { id: 'q1', artifact_id: 'a1', state: 'in_review' }, error: null },
			{ data: null, error: null },
			{ data: null, error: null }
		]);
		const res = await decideTriage(db, 't1', 'sig', { status: 'decline', reason: 'dup' }, 'm1');
		expect(res).toEqual({ status: 'declined' });
	});

	it('revise → in_review, revised', async () => {
		const { db, calls } = makeDb([
			{ data: { id: 'q1', artifact_id: 'a1', state: 'queued' }, error: null },
			{ data: null, error: null },
			{ data: null, error: null }
		]);
		const res = await decideTriage(db, 't1', 'sig', { status: 'revise' }, 'm1');
		expect(res).toEqual({ status: 'revised' });
		const upd = calls.find((c) => c.table === 'triage_queue' && c.op === 'update');
		expect(upd?.payload).toMatchObject({ state: 'in_review' });
	});

	it('throws 500 when the decisions insert fails', async () => {
		const { db } = makeDb([
			{ data: { id: 'q1', artifact_id: 'a1', state: 'queued' }, error: null },
			{ data: null, error: { message: 'insert boom' } }
		]);
		await expect(
			decideTriage(db, 't1', 'sig', { status: 'revise' }, 'm1')
		).rejects.toMatchObject({ status: 500 });
	});
});
