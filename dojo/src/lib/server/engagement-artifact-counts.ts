// Per-engagement artifact tally for the lead Engagements register — the real
// source of each row's "N lessons kept · M stripped" (previously hardcoded 0 in
// the mapper). Aggregates `dojo.artifacts` by engagement + status:
//   lessonsKept = status 'published' — approved & live (what crossed the boundary)
//   stripped    = status 'archived'  — declined/retired (held back, did not cross)
// ('submitted' is in-flight — neither, per dojo.artifact_status semantics.) This
// is a compliance-facing count, so it fails closed (a query error throws, never a
// fabricated/partial number) — see the no-fabrication rule.
import { AdminError, type DojoClient } from './admin-data';

/** Kept-vs-stripped tally for one engagement. */
export interface EngagementCounts {
	lessonsKept: number;
	stripped: number;
}

/** The two artifact statuses that resolve to a kept/stripped outcome. */
const COUNTED_STATUSES = ['published', 'archived'];

/**
 * Tally a flat `{ engagement_id, status }` projection into per-engagement counts
 * (pure). Rows with a null engagement_id, or a status other than published/archived,
 * are ignored (defensive — the query already filters, but the tally is the guard).
 */
export function tallyByEngagement(
	rows: { engagement_id: string | null; status: string }[]
): Map<string, EngagementCounts> {
	const out = new Map<string, EngagementCounts>();
	for (const r of rows) {
		if (!r.engagement_id) continue;
		const c = out.get(r.engagement_id) ?? { lessonsKept: 0, stripped: 0 };
		if (r.status === 'published') c.lessonsKept++;
		else if (r.status === 'archived') c.stripped++;
		else continue; // submitted / unknown → not a kept/stripped outcome
		out.set(r.engagement_id, c);
	}
	return out;
}

/**
 * Count published (kept) vs archived (stripped) artifacts per engagement, for the
 * given tenant + engagement ids. One bounded aggregate query (grouped in JS).
 * Returns a Map keyed by engagement_id — an engagement with no counted artifacts is
 * simply absent (the caller reads it as 0/0). Fail-closed: a query error throws
 * AdminError(500), never a partial tally.
 */
export async function countEngagementArtifacts(
	db: DojoClient,
	tenantId: string,
	engagementIds: string[]
): Promise<Map<string, EngagementCounts>> {
	const ids = [...new Set(engagementIds.filter((id) => typeof id === 'string' && id.length > 0))];
	if (ids.length === 0) return new Map();
	const { data, error } = await db
		.from('artifacts')
		.select('engagement_id, status')
		.eq('tenant_id', tenantId)
		.in('engagement_id', ids)
		.in('status', COUNTED_STATUSES);
	if (error) throw new AdminError(500, error.message);
	return tallyByEngagement((data ?? []) as { engagement_id: string | null; status: string }[]);
}
