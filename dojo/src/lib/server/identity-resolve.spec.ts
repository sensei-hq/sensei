// Unit tests for the WS-1 identity resolver (`identity-resolve.ts`): the pure
// best-identity pick and the tenant-scoped `resolveDisplayNames` DB wrapper
// (dedup, grouping, fail-closed on error, no-fabrication on a miss).
import { describe, it, expect } from 'vitest';
import { pickBestIdentity, resolveDisplayNames, type IdentityNameRow } from './identity-resolve';
import { AdminError, type DojoClient } from './admin-data';

function row(over: Partial<IdentityNameRow> = {}): IdentityNameRow {
	return { principal_id: 'u1', display_name: null, email: null, last_login_at: null, ...over };
}

describe('pickBestIdentity', () => {
	it('returns null/null for no rows', () => {
		expect(pickBestIdentity([])).toEqual({ display_name: null, email: null });
	});
	it('returns the name + email of a single row', () => {
		expect(pickBestIdentity([row({ display_name: 'Ada', email: 'ada@x.co' })])).toEqual({
			display_name: 'Ada',
			email: 'ada@x.co'
		});
	});
	it('prefers a named row over an email-only row over a bare row', () => {
		const rows = [row({ email: 'only@x.co' }), row({}), row({ display_name: 'Grace', email: 'g@x.co' })];
		expect(pickBestIdentity(rows)).toEqual({ display_name: 'Grace', email: 'g@x.co' });
	});
	it('within the same tier, the most recent last_login_at wins', () => {
		const rows = [
			row({ display_name: 'Old', last_login_at: '2026-01-01T00:00:00Z' }),
			row({ display_name: 'New', last_login_at: '2026-07-01T00:00:00Z' })
		];
		expect(pickBestIdentity(rows).display_name).toBe('New');
	});
	it('treats a null last_login_at as oldest', () => {
		const rows = [
			row({ display_name: 'HasLogin', last_login_at: '2026-01-01T00:00:00Z' }),
			row({ display_name: 'NoLogin', last_login_at: null })
		];
		expect(pickBestIdentity(rows).display_name).toBe('HasLogin');
	});
	it('treats a whitespace-only display_name as absent (falls back to email)', () => {
		expect(pickBestIdentity([row({ display_name: '   ', email: 'e@x.co' })])).toEqual({
			display_name: null,
			email: 'e@x.co'
		});
	});
	it('returns null/null when every row is bare', () => {
		expect(pickBestIdentity([row({}), row({})])).toEqual({ display_name: null, email: null });
	});
});

// A stub whose `.from().select().eq().in()` terminal resolves the given result,
// capturing the `in()` user-id list.
function makeDb(result: { data?: unknown; error: unknown }) {
	const captured: { ids?: unknown } = {};
	const b: Record<string, unknown> = {};
	b.from = () => b;
	b.select = () => b;
	b.eq = () => b;
	b.in = (_col: string, ids: unknown) => {
		captured.ids = ids;
		return Promise.resolve(result);
	};
	return { db: b as unknown as DojoClient, captured };
}

describe('resolveDisplayNames', () => {
	it('returns an empty map (no query) for an empty id list', async () => {
		const { db, captured } = makeDb({ data: [], error: null });
		expect(await resolveDisplayNames(db, 't1', [])).toEqual(new Map());
		expect(captured.ids).toBeUndefined(); // never queried
	});
	it('dedups user ids before querying', async () => {
		const { db, captured } = makeDb({ data: [], error: null });
		await resolveDisplayNames(db, 't1', ['u1', 'u1', 'u2', '']);
		expect(captured.ids).toEqual(['u1', 'u2']); // deduped + blanks dropped
	});
	it('groups multiple identity rows per principal and picks the best', async () => {
		const { db } = makeDb({
			data: [
				{ principal_id: 'u1', display_name: null, email: 'u1@x.co', last_login_at: null },
				{ principal_id: 'u1', display_name: 'Ada', email: null, last_login_at: '2026-07-01T00:00:00Z' },
				{ principal_id: 'u2', display_name: 'Bob', email: 'bob@x.co', last_login_at: null }
			],
			error: null
		});
		const map = await resolveDisplayNames(db, 't1', ['u1', 'u2']);
		expect(map.get('u1')).toEqual({ display_name: 'Ada', email: null });
		expect(map.get('u2')).toEqual({ display_name: 'Bob', email: 'bob@x.co' });
	});
	it('omits users with no identity row (caller falls back to shortId)', async () => {
		const { db } = makeDb({ data: [{ principal_id: 'u1', display_name: 'Ada', email: null, last_login_at: null }], error: null });
		const map = await resolveDisplayNames(db, 't1', ['u1', 'ghost']);
		expect(map.has('ghost')).toBe(false);
	});
	it('treats null data as no identities', async () => {
		const { db } = makeDb({ data: null, error: null });
		expect(await resolveDisplayNames(db, 't1', ['u1'])).toEqual(new Map());
	});
	it('throws AdminError(500) on a query error (fail-closed, no fabrication)', async () => {
		const { db } = makeDb({ data: null, error: { message: 'boom' } });
		await expect(resolveDisplayNames(db, 't1', ['u1'])).rejects.toBeInstanceOf(AdminError);
	});
});
