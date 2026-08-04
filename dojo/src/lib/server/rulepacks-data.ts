// The global rule-pack LIBRARY read (browse). Lists the curated, globally-visible
// packs a user can adopt — seeded via the staging-file model (import_rule_packs),
// owner_namespace_id NULL + status 'active'. User-wide (/v1/you plane): the catalog
// is the same for everyone; per-caller adoption state is layered on separately (see
// listAdoptedPackSlugs). Read-only; fails closed via the route; honest-empty until
// the library is seeded — never a fixture.

import type { DojoClient } from './rules-data';
import type { LibraryPackWire } from '$lib/client-data';

/** A row of the `dojo.rule_pack_library` view — browse fields + rules pre-aggregated
 *  (jsonb array of statements, ordinal order) by the view. */
interface LibraryPackRow {
	slug: string;
	kanji: string | null;
	name: string;
	source: string;
	summary: string | null;
	rules: string[] | null;
}

/** The select for the library browse (the view already filters + orders rules). */
export const LIBRARY_SELECT = 'slug, kanji, name, source, summary, rules';

/** Shape the view rows into the wire form. Pure over its input so the wire-shape is
 *  unit-tested directly. Rules arrive pre-ordered from the view; pass them through. */
export function shapeLibraryPacks(rows: LibraryPackRow[]): LibraryPackWire[] {
	return rows.map((r) => ({
		slug: r.slug,
		kanji: r.kanji ?? null,
		name: r.name,
		by: r.source,
		note: r.summary ?? '',
		rules: Array.isArray(r.rules) ? r.rules : []
	}));
}

/**
 * List the global rule-pack library via `dojo.rule_pack_library` — a dojo-schema
 * view over the sensei rule-pack tables (already filtered to global + active, rules
 * ordinal-aggregated), so the API reads NO `sensei.*` table directly (no grant on
 * the shared plane). Ordered by name. A read error throws (the route maps it to
 * 500); an empty library is honest-empty `[]`, never a fixture.
 */
export async function listLibraryPacks(db: DojoClient): Promise<LibraryPackWire[]> {
	const { data, error } = await db
		.from('rule_pack_library')
		.select(LIBRARY_SELECT)
		.order('name');
	if (error) throw new RulePacksError(500, error.message);
	return shapeLibraryPacks((data ?? []) as unknown as LibraryPackRow[]);
}

/** A domain error carrying the HTTP status the route should return. */
export class RulePacksError extends Error {
	constructor(
		readonly status: number,
		message: string
	) {
		super(message);
	}
}
