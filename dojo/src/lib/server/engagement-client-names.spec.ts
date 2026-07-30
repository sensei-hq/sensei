// Unit tests for the engagement client-name resolver (`engagement-client-names.ts`):
// dedup, name mapping, no-fabrication on a miss, fail-closed on error.
import { describe, it, expect } from 'vitest';
import { resolveEngagementClientNames } from './engagement-client-names';
import { AdminError, type DojoClient } from './admin-data';

// A stub whose `.from().select().eq().in()` terminal resolves the result,
// capturing the `in()` id list.
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

describe('resolveEngagementClientNames', () => {
	it('returns an empty map (no query) for no ids', async () => {
		const { db, captured } = makeDb({ data: [], error: null });
		expect(await resolveEngagementClientNames(db, 't1', [])).toEqual(new Map());
		expect(captured.ids).toBeUndefined();
	});
	it('dedups ids and drops blanks before querying', async () => {
		const { db, captured } = makeDb({ data: [], error: null });
		await resolveEngagementClientNames(db, 't1', ['e1', 'e1', 'e2', '']);
		expect(captured.ids).toEqual(['e1', 'e2']);
	});
	it('maps each engagement id to its client_name', async () => {
		const { db } = makeDb({
			data: [
				{ id: 'e1', client_name: 'Globex' },
				{ id: 'e2', client_name: 'Initech' }
			],
			error: null
		});
		const m = await resolveEngagementClientNames(db, 't1', ['e1', 'e2']);
		expect(m.get('e1')).toBe('Globex');
		expect(m.get('e2')).toBe('Initech');
	});
	it('omits engagements with a null/blank client_name (caller falls back)', async () => {
		const { db } = makeDb({ data: [{ id: 'e1', client_name: null }], error: null });
		const m = await resolveEngagementClientNames(db, 't1', ['e1']);
		expect(m.has('e1')).toBe(false);
	});
	it('treats null data as no engagements', async () => {
		const { db } = makeDb({ data: null, error: null });
		expect(await resolveEngagementClientNames(db, 't1', ['e1'])).toEqual(new Map());
	});
	it('throws AdminError(500) on a query error (fail-closed)', async () => {
		const { db } = makeDb({ data: null, error: { message: 'boom' } });
		await expect(resolveEngagementClientNames(db, 't1', ['e1'])).rejects.toBeInstanceOf(AdminError);
	});
});
