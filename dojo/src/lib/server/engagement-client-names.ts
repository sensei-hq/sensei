// Resolve a set of engagement ids → their client display name, for surfaces that
// reference an engagement but only carry its id (e.g. an incident's engagement_id,
// which rendered as a truncated uuid). One bounded query over dojo.engagements
// (Rule C: the name lives in `client_name`). Fail-closed: a query error throws
// AdminError(500), never a fabricated name; an engagement with no row is simply
// absent from the map (the caller falls back to a short id / '—').
import { AdminError, type DojoClient } from './admin-data';

/**
 * Map each engagement id (within a tenant) to its `client_name`. Returns a Map —
 * only engagements that exist appear; the caller reads a miss as "no name".
 */
export async function resolveEngagementClientNames(
	db: DojoClient,
	tenantId: string,
	engagementIds: string[]
): Promise<Map<string, string>> {
	const ids = [...new Set(engagementIds.filter((id) => typeof id === 'string' && id.length > 0))];
	if (ids.length === 0) return new Map();
	const { data, error } = await db.from('engagements').select('id, client_name').eq('tenant_id', tenantId).in('id', ids);
	if (error) throw new AdminError(500, error.message);
	const out = new Map<string, string>();
	for (const r of (data ?? []) as { id: string; client_name: string | null }[]) {
		if (r.client_name) out.set(r.id, r.client_name);
	}
	return out;
}
