// Map the library-pack wire shape (GET /v1/you/rule-packs) onto the KitRulePack the
// shipped ScrRulePacks screen renders. Pure over plain data so it unit-tests without
// a DOM. The `adopted` flag is layered from the caller's adoptions (empty until the
// adopt endpoint is wired — a fresh library shows everything as not-yet-adopted,
// never a fabricated adoption).

import type { LibraryPackWire } from './client-data';
import type { KitRulePack } from './components/kit/types';

/** Fallback glyph when a pack carries no kanji. */
const DEFAULT_KANJI = '守';

/** Library wire packs → `KitRulePack[]`. `adoptedSlugs` marks which the caller has
 *  adopted (defaults to none). `id` is the stable slug; rule count = rules.length. */
export function toKitRulePacks(
	packs: LibraryPackWire[],
	adoptedSlugs: ReadonlySet<string> = new Set()
): KitRulePack[] {
	return packs.map((p) => ({
		id: p.slug,
		kanji: p.kanji && p.kanji.trim() !== '' ? p.kanji : DEFAULT_KANJI,
		name: p.name,
		by: p.by,
		note: p.note,
		rules: p.rules,
		adopted: adoptedSlugs.has(p.slug)
	}));
}
