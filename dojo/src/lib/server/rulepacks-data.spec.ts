import { describe, it, expect } from 'vitest';
import {
	shapeLibraryPacks,
	listLibraryPacks,
	listAdoptedPackSlugs,
	setPackAdoption,
	RulePacksError
} from './rulepacks-data';

// The DojoClient param type, derived from a function under test so the stub can
// never drift from the real signature.
type DbArg = Parameters<typeof listLibraryPacks>[0];

/**
 * A chainable Supabase-query stub. Every builder method (`from`/`select`/`eq`/
 * `order`) returns the same object; the object is itself thenable so `await`-ing
 * the end of ANY chain (whether it terminates on `.order()` or on `.eq()`)
 * resolves the supplied `{ data, error }`. `.rpc()` resolves it directly.
 */
type Result = { data?: unknown; error: unknown };
function makeDb(result: Result): DbArg {
	const b: Record<string, unknown> = {};
	for (const m of ['from', 'select', 'eq', 'order', 'limit']) b[m] = () => b;
	b.then = (resolve: (r: Result) => unknown) => resolve(result);
	b.rpc = () => Promise.resolve(result);
	return b as unknown as DbArg;
}

describe('shapeLibraryPacks — view rows → library wire', () => {
	it('maps view fields (source→by, summary→note) and passes rules through', () => {
		const [p] = shapeLibraryPacks([
			{ slug: 'x', kanji: '技', name: 'X', source: 'src', summary: 's', rules: ['first', 'second'] }
		]);
		expect(p.rules).toEqual(['first', 'second']); // view already ordered by ordinal
		expect(p.by).toBe('src');
		expect(p.note).toBe('s');
		expect(p.kanji).toBe('技');
	});

	it('handles null summary / kanji / rules as honest empties', () => {
		const [p] = shapeLibraryPacks([
			{ slug: 'y', kanji: null, name: 'Y', source: 'src', summary: null, rules: null }
		]);
		expect(p.note).toBe('');
		expect(p.kanji).toBeNull();
		expect(p.rules).toEqual([]);
	});
});

describe('listLibraryPacks — DB read → shaped wire', () => {
	it('shapes the rows returned by the library view', async () => {
		const rows = [{ slug: 'x', kanji: '技', name: 'X', source: 'src', summary: 's', rules: ['a'] }];
		expect(await listLibraryPacks(makeDb({ data: rows, error: null }))).toEqual([
			{ slug: 'x', kanji: '技', name: 'X', by: 'src', note: 's', rules: ['a'] }
		]);
	});

	it('returns honest-empty [] when the library is unseeded (data null)', async () => {
		expect(await listLibraryPacks(makeDb({ data: null, error: null }))).toEqual([]);
	});

	it('throws RulePacksError(500) on a read error', async () => {
		await expect(listLibraryPacks(makeDb({ data: null, error: { message: 'boom' } }))).rejects.toThrow(
			RulePacksError
		);
	});
});

describe('listAdoptedPackSlugs — user-scoped adoption read', () => {
	it('maps rows to their pack_slug', async () => {
		const db = makeDb({ data: [{ pack_slug: 'a' }, { pack_slug: 'b' }], error: null });
		expect(await listAdoptedPackSlugs(db, 'user-1')).toEqual(['a', 'b']);
	});

	it('returns [] when the caller has adopted nothing (data null)', async () => {
		expect(await listAdoptedPackSlugs(makeDb({ data: null, error: null }), 'user-1')).toEqual([]);
	});

	it('throws RulePacksError on a read error', async () => {
		await expect(
			listAdoptedPackSlugs(makeDb({ data: null, error: { message: 'boom' } }), 'user-1')
		).rejects.toThrow(RulePacksError);
	});
});

describe('setPackAdoption — adopt/drop via SECURITY DEFINER rpc', () => {
	it('returns true when the definer function reports the pack existed', async () => {
		const db = makeDb({ data: true, error: null });
		expect(await setPackAdoption(db, 'slug', 'user-1', 'Ada', true)).toBe(true);
	});

	it('returns false for an unknown/unavailable slug (data false)', async () => {
		const db = makeDb({ data: false, error: null });
		expect(await setPackAdoption(db, 'nope', 'user-1', null, true)).toBe(false);
	});

	it('throws RulePacksError on an rpc error', async () => {
		const db = makeDb({ data: null, error: { message: 'boom' } });
		await expect(setPackAdoption(db, 'slug', 'user-1', null, false)).rejects.toThrow(RulePacksError);
	});
});
