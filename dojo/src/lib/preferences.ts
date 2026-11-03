// A person's display preferences (#133), shared by the dōjō's server and its
// browser code: the glyph mode and when it was chosen. The later choice wins —
// by when it was made, not when it arrived — so a sync in either direction
// converges and never undoes a newer choice.
export type GlyphMode = 'kanji' | 'icon';
export interface GlyphPref {
	glyphs: GlyphMode;
	/** When the person chose it; null: never chosen, so any choice wins. */
	glyphs_set_at: string | null;
}

/** The choice made later; one never made loses to any. */
export function laterChoice(a: GlyphPref, b: GlyphPref): GlyphPref {
	if (!a.glyphs_set_at) return b;
	if (!b.glyphs_set_at) return a;
	return Date.parse(b.glyphs_set_at) > Date.parse(a.glyphs_set_at) ? b : a;
}
