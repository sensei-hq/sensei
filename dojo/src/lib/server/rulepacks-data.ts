// The global rule-pack LIBRARY read (browse). Lists the curated, globally-visible
// packs a user can adopt — seeded via the staging-file model (import_rule_packs),
// owner_namespace_id NULL + status 'active'. User-wide (/v1/you plane): the catalog
// is the same for everyone; per-caller adoption state is layered on separately (see
// listAdoptedPackSlugs). Read-only; fails closed via the route; honest-empty until
// the library is seeded — never a fixture.

import type { DojoClient } from './rules-data';
import type { LibraryPackWire } from '$lib/client-data';

/** A `rule_packs ⋈ rule_pack_rules` row from the browse select (embedded rules). */
interface LibraryPackRow {
	slug: string;
	kanji: string | null;
	name: string;
	source: string;
	summary: string | null;
	rule_pack_rules: { statement: string; ordinal: number }[] | null;
}

/** The select for the library browse — global packs + their embedded rule statements. */
export const LIBRARY_SELECT = 'slug, kanji, name, source, summary, rule_pack_rules(statement, ordinal)';

/** Shape the joined rows into the wire form: rules flattened to statements in
 *  ordinal order. Pure over its input so the wire-shape is unit-tested directly. */
export function shapeLibraryPacks(rows: LibraryPackRow[]): LibraryPackWire[] {
	return rows.map((r) => ({
		slug: r.slug,
		kanji: r.kanji ?? null,
		name: r.name,
		by: r.source,
		note: r.summary ?? '',
		rules: [...(r.rule_pack_rules ?? [])]
			.sort((a, b) => a.ordinal - b.ordinal)
			.map((x) => x.statement)
	}));
}

/**
 * List the global rule-pack library: `owner_namespace_id IS NULL` (curated global,
 * never an org's private pack) AND `status = 'active'` (visible), with each pack's
 * rule statements. Ordered by name. A read error throws (the route maps it to 500);
 * an empty library is honest-empty `[]`, never a fixture.
 */
export async function listLibraryPacks(db: DojoClient): Promise<LibraryPackWire[]> {
	const { data, error } = await db
		.schema('sensei')
		.from('rule_packs')
		.select(LIBRARY_SELECT)
		.is('owner_namespace_id', null)
		.eq('status', 'active')
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
