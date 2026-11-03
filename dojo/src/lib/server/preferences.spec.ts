import { describe, it, expect } from 'vitest';
import { readPreferences, writeGlyphs, laterChoice } from './preferences';

/** A one-table fake of the supabase-js client, enough for these reads/writes. */
function fakeDb(row: Record<string, unknown> | null, fail?: string) {
	const state = { row, upserts: [] as Record<string, unknown>[] };
	const db = {
		from: () => ({
			select: () => ({
				eq: () => ({
					maybeSingle: async () => (fail ? { data: null, error: { message: fail } } : { data: state.row, error: null })
				})
			}),
			upsert: (r: Record<string, unknown>) => {
				state.upserts.push(r);
				state.row = r;
				return { select: () => ({ single: async () => ({ data: r, error: null }) }) };
			}
		})
	};
	return { db: db as never, state };
}

describe('laterChoice', () => {
	it('prefers the choice made later; any choice beats one never made', () => {
		const a = { glyphs: 'icon' as const, glyphs_set_at: '2026-10-27T10:00:00Z' };
		const b = { glyphs: 'kanji' as const, glyphs_set_at: '2026-10-27T09:00:00Z' };
		expect(laterChoice(a, b)).toBe(a);
		expect(laterChoice(b, a)).toBe(a);
		expect(laterChoice({ glyphs: 'kanji', glyphs_set_at: null }, b)).toBe(b);
	});
});

describe('preferences', () => {
	it('reads kanji, never chosen, for a person with no row', async () => {
		expect(await readPreferences(fakeDb(null).db, 'p1')).toEqual({ glyphs: 'kanji', glyphs_set_at: null });
	});

	it('fails rather than answering a default when the read fails', async () => {
		await expect(readPreferences(fakeDb(null, 'boom').db, 'p1')).rejects.toThrow('boom');
	});

	it('writes a later choice and keeps a later stored one', async () => {
		const { db, state } = fakeDb({ glyphs: 'icon', glyphs_set_at: '2026-10-27T10:00:00Z' });
		expect(await writeGlyphs(db, 'p1', { glyphs: 'kanji', glyphs_set_at: '2026-10-27T09:00:00Z' })).toEqual({
			glyphs: 'icon', glyphs_set_at: '2026-10-27T10:00:00Z'
		});
		expect(state.upserts).toHaveLength(0);
		expect(await writeGlyphs(db, 'p1', { glyphs: 'kanji', glyphs_set_at: '2026-10-27T11:00:00Z' })).toEqual({
			glyphs: 'kanji', glyphs_set_at: '2026-10-27T11:00:00Z'
		});
		expect(state.upserts[0]).toMatchObject({ user_id: 'p1', glyphs: 'kanji', glyphs_set_at: '2026-10-27T11:00:00Z' });
	});
});
