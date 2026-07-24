// Unit tests for the admin-console store logic (`admin-data.ts`). Exercises:
//   • listMembers / listIdentities / listPolicies / listAudit — the tenant
//     filter + envelope rows + error → AdminError(500), and the audit limit
//     clamp.
//   • getHealth — the four isolated count queries composed into the rollup, and
//     a count-query error surfacing (never a silent 0).
import { describe, it, expect } from 'vitest';
import {
	listMembers,
	listIdentities,
	listPolicies,
	listAudit,
	getHealth,
	AdminError,
	type DojoClient
} from './admin-data';

type Terminal = { data?: unknown; count?: number | null; error: unknown };

/** A stub whose terminal (`.order()` / `.limit()`) resolves the given result. */
function makeListDb(result: Terminal) {
	const captured: { limit?: number } = {};
	const b: Record<string, unknown> = {};
	b.from = () => b;
	b.select = () => b;
	b.eq = () => b;
	b.order = () => b;
	b.limit = (n: number) => {
		captured.limit = n;
		return Promise.resolve(result);
	};
	// list*() without a limit awaits `.order()` directly.
	b.then = (resolve: (v: Terminal) => unknown) => resolve(result);
	return { db: b as unknown as DojoClient, captured };
}

describe('listMembers / listIdentities / listPolicies', () => {
	it('returns the rows on success', async () => {
		const rows = [{ id: 'm1' }];
		expect(await listMembers(makeListDb({ data: rows, error: null }).db, 't1')).toEqual(rows);
		expect(await listIdentities(makeListDb({ data: rows, error: null }).db, 't1')).toEqual(rows);
		expect(await listPolicies(makeListDb({ data: rows, error: null }).db, 't1')).toEqual(rows);
	});
	it('returns [] when data is null', async () => {
		expect(await listMembers(makeListDb({ data: null, error: null }).db, 't1')).toEqual([]);
	});
	it('throws AdminError(500) on a query error', async () => {
		await expect(listMembers(makeListDb({ data: null, error: { message: 'boom' } }).db, 't1')).rejects.toThrow(AdminError);
	});
});

describe('listAudit', () => {
	it('clamps the limit to 1..500 (default 100)', async () => {
		const d1 = makeListDb({ data: [], error: null });
		await listAudit(d1.db, 't1');
		expect(d1.captured.limit).toBe(100);
		const d2 = makeListDb({ data: [], error: null });
		await listAudit(d2.db, 't1', 9999);
		expect(d2.captured.limit).toBe(500);
		const d3 = makeListDb({ data: [], error: null });
		await listAudit(d3.db, 't1', 0);
		expect(d3.captured.limit).toBe(1);
	});
});

// ── getHealth: four sequential count queries ─────────────────────────────────
// Each `.from(...).select(..,{head}).eq(...).gte()/.in()/.eq()` chain resolves
// (awaited) to the next `{ count, error }` in order.
function makeCountDb(results: Terminal[]) {
	let i = 0;
	const next = (): Terminal => results[i++] ?? { count: 0, error: null };
	const b: Record<string, unknown> = {};
	b.from = () => b;
	b.select = () => b;
	b.eq = () => b;
	b.in = () => b;
	b.gte = () => Promise.resolve(next());
	// The error_rate query ends on `.eq()` (no gte/in) — resolve on await.
	b.then = (resolve: (v: Terminal) => unknown) => resolve(next());
	return b as unknown as DojoClient;
}

describe('getHealth', () => {
	it('composes the four counts into the rollup', async () => {
		const db = makeCountDb([
			{ count: 3, error: null }, // connections (.gte heartbeat)
			{ count: 12, error: null }, // queue_depth (.eq state) — resolves via .then
			{ count: 5, error: null }, // publish_rate (.gte ts)
			{ count: 1, error: null } // error_rate (.eq sync_status) — via .then
		]);
		const h = await getHealth(db, 't1');
		expect(h).toEqual({ connections: 3, queue_depth: 12, publish_rate_1h: 5, error_rate_1h: 1 });
	});
	it('surfaces a count-query error rather than blanking the strip', async () => {
		const db = makeCountDb([{ count: null, error: { message: 'count boom' } }]);
		await expect(getHealth(db, 't1')).rejects.toThrow(AdminError);
	});
});
