// The person's display preferences (#133), read and written as them.
import { laterChoice, type GlyphPref } from '$lib/preferences';

export { laterChoice, type GlyphPref };

// supabase-js's builder is untyped here: the route passes the caller's client.
// eslint-disable-next-line @typescript-eslint/no-explicit-any
type Db = any;

const NEVER: GlyphPref = { glyphs: 'kanji', glyphs_set_at: null };

/** The person's preferences; kanji, never chosen, when they have none. A
 *  failed read throws — it is never answered with the default. */
export async function readPreferences(db: Db, userId: string): Promise<GlyphPref> {
	const { data, error } = await db
		.from('user_preferences')
		.select('glyphs, glyphs_set_at')
		.eq('user_id', userId)
		.maybeSingle();
	if (error) throw new Error(error.message);
	return data ? { glyphs: data.glyphs, glyphs_set_at: data.glyphs_set_at } : NEVER;
}

/** Store a choice unless a later one is stored; answers what is stored. */
export async function writeGlyphs(db: Db, userId: string, choice: GlyphPref): Promise<GlyphPref> {
	const stored = await readPreferences(db, userId);
	const winner = laterChoice(stored, choice);
	if (winner === stored) return stored;
	const { error } = await db
		.from('user_preferences')
		.upsert(
			{ user_id: userId, glyphs: choice.glyphs, glyphs_set_at: choice.glyphs_set_at, updated_at: new Date().toISOString() },
			{ onConflict: 'user_id' }
		)
		.select('user_id')
		.single();
	if (error) throw new Error(error.message);
	return choice;
}
