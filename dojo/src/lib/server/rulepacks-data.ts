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

/**
 * The slugs of library packs the caller has adopted — read from the
 * `dojo.pack_adoption` view (a dojo-schema view over the sensei adoption tables,
 * owner privileges → no sensei grant), filtered to the caller's USER-scoped
 * namespace (scope_key 'user', slug = user id). Honest-empty `[]` on none; a read
 * error throws (the route maps it to 500).
 */
export async function listAdoptedPackSlugs(db: DojoClient, userId: string): Promise<string[]> {
	const { data, error } = await db
		.from('pack_adoption')
		.select('pack_slug')
		.eq('scope_key', 'user')
		.eq('namespace_slug', userId);
	if (error) throw new RulePacksError(500, error.message);
	return ((data ?? []) as { pack_slug: string }[]).map((r) => r.pack_slug);
}

/**
 * Adopt (or drop) a library pack for the caller's USER-scoped namespace, via the
 * `dojo.set_pack_adoption` SECURITY DEFINER function (writes sensei.* with owner
 * privileges — no sensei grant). Returns whether the pack existed (false → the
 * route 404s an unknown/unavailable slug). A call error throws (→ 500).
 */
export async function setPackAdoption(
	db: DojoClient,
	slug: string,
	userId: string,
	userName: string | null,
	adopt: boolean
): Promise<boolean> {
	const { data, error } = await db.rpc('set_pack_adoption', {
		p_pack_slug: slug,
		p_user_id: userId,
		p_user_name: userName ?? '',
		p_adopt: adopt
	});
	if (error) throw new RulePacksError(500, error.message);
	return data === true;
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
