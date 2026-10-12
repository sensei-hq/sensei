import { describe, it, expect } from 'vitest';
import {
	shapeLibraryPacks,
	LIBRARY_SELECT,
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
const rpcCalls: { fn: string; args?: Record<string, unknown> }[] = [];
function makeDb(result: Result): DbArg {
	rpcCalls.length = 0;
	const b: Record<string, unknown> = {};
	for (const m of ['from', 'select', 'eq', 'order', 'limit']) b[m] = () => b;
	b.then = (resolve: (r: Result) => unknown) => resolve(result);
	b.rpc = (fn: string, args?: Record<string, unknown>) => {
		rpcCalls.push({ fn, args });
		return Promise.resolve(result);
	};
	return b as unknown as DbArg;
}

describe('shapeLibraryPacks — view rows → library wire', () => {
	// dojo.rule_pack_library names the column `attribution` (since 13efa1bd); the
	// read selected `source`, so PostgREST refused the whole library (42703) and
	// GET /v1/you/rule-packs 500'd. The select must name the view's columns.
	it('selects the view\'s columns, as the view names them', () => {
		expect(LIBRARY_SELECT).toBe('slug, kanji, name, attribution, summary, rules');
	});

	it('maps view fields (attribution→by, summary→note) and passes rules through', () => {
		const [p] = shapeLibraryPacks([
			{ slug: 'x', kanji: '技', name: 'X', attribution: 'src', summary: 's', rules: ['first', 'second'] }
		]);
		expect(p.rules).toEqual(['first', 'second']); // view already ordered by ordinal
		expect(p.by).toBe('src');
		expect(p.note).toBe('s');
		expect(p.kanji).toBe('技');
	});

	it('handles null summary / kanji / rules as honest empties', () => {
		const [p] = shapeLibraryPacks([
			{ slug: 'y', kanji: null, name: 'Y', attribution: 'src', summary: null, rules: null }
		]);
		expect(p.note).toBe('');
		expect(p.kanji).toBeNull();
		expect(p.rules).toEqual([]);
	});
});

describe('listLibraryPacks — DB read → shaped wire', () => {
	it('shapes the rows returned by the library view', async () => {
		const rows = [{ slug: 'x', kanji: '技', name: 'X', attribution: 'src', summary: 's', rules: ['a'] }];
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

// The caller's own adoptions, pinned to them in the database by
// dojo.my_adopted_pack_slugs() (#236; database/tests/dojo/rule_packs_access.sql).
describe('listAdoptedPackSlugs — the caller\'s own adoptions', () => {
	it('reads them through dojo.my_adopted_pack_slugs(), with no user to pass', async () => {
		const db = makeDb({ data: ['a', 'b'], error: null });
		expect(await listAdoptedPackSlugs(db)).toEqual(['a', 'b']);
		expect(rpcCalls).toEqual([{ fn: 'my_adopted_pack_slugs', args: undefined }]);
	});

	it('returns [] when the caller has adopted nothing (data null)', async () => {
		expect(await listAdoptedPackSlugs(makeDb({ data: null, error: null }))).toEqual([]);
	});

	it('throws RulePacksError on a read error', async () => {
		await expect(listAdoptedPackSlugs(makeDb({ data: null, error: { message: 'boom' } }))).rejects.toThrow(
			RulePacksError
		);
	});
});

describe('setPackAdoption — adopt/drop via SECURITY DEFINER rpc', () => {
	// The function derives the user from the session; a user id in the call is
	// what let a caller of the old signature adopt into anyone's namespace (#236).
	it('passes the pack, the display name and the choice — never a user id', async () => {
		const db = makeDb({ data: true, error: null });
		expect(await setPackAdoption(db, 'slug', 'Ada', true)).toBe(true);
		expect(rpcCalls).toEqual([
			{ fn: 'set_pack_adoption', args: { p_pack_slug: 'slug', p_user_name: 'Ada', p_adopt: true } }
		]);
	});

	it('returns false for an unknown/unavailable slug (data false)', async () => {
		const db = makeDb({ data: false, error: null });
		expect(await setPackAdoption(db, 'nope', null, true)).toBe(false);
	});

	it('throws RulePacksError on an rpc error', async () => {
		const db = makeDb({ data: null, error: { message: 'boom' } });
		await expect(setPackAdoption(db, 'slug', null, false)).rejects.toThrow(RulePacksError);
	});
});
