// The lead Client-audit CONFIDENTIALITY/CONTAINMENT ledger (`GET …/audit/ledger`)
// — `dojo.audit_events` filtered to the confidentiality events (NOT the general
// admin action-audit the screen was wrongly bound to), each enriched with the
// engagement's client name (Rule C). Semantics (client-audit.md resolved design):
//   • publish            → a lesson crossed the boundary (source-stripped by
//                          construction — the always-on invariant)
//   • contained / held   → the guard blocked a would-be leak (the guard WORKING,
//                          not a failure — emitted by the health containment seam)
// There is no "broken strip" red-fail here: every row is confidentiality holding.
import type { dojoDb } from './dojo-supabase';
import { AdminError, type DojoClient } from './admin-data';
import { resolveEngagementClientNames } from './engagement-client-names';

export type { DojoClient };
export { AdminError };

/** The audit actions that make up the confidentiality ledger. `contained`/`held`
 *  are emitted by the containment seam (health screen); forward-compatible. */
export const CONFIDENTIALITY_ACTIONS = ['publish', 'contained', 'held'];

/** One confidentiality-ledger row (an audit event + resolved client name). */
export interface ClientAuditEntry {
	id: number;
	ts: string;
	action: string;
	target: string | null;
	detail: unknown;
	engagement_id: string | null;
	/** Engagement → client_name (Rule C), or null when unbound/unresolved. */
	client_name: string | null;
}

const LEDGER_COLS = 'id, ts, action, target, detail, engagement_id';

/**
 * Read the tenant's confidentiality ledger, most recent first, capped at `limit`
 * (default 200, clamped 1..500), enriched with each engagement's client name.
 * Fails closed (AdminError 500) — never a fabricated ledger.
 */
export async function getClientAuditLedger(
	db: DojoClient,
	tenantId: string,
	limit = 200
): Promise<ClientAuditEntry[]> {
	const capped = Math.max(1, Math.min(500, Number.isFinite(limit) ? limit : 200));
	const { data, error } = await db
		.from('audit_events')
		.select(LEDGER_COLS)
		.eq('tenant_id', tenantId)
		.in('action', CONFIDENTIALITY_ACTIONS)
		.order('ts', { ascending: false })
		.limit(capped);
	if (error) throw new AdminError(500, error.message);
	const rows = (data ?? []) as Omit<ClientAuditEntry, 'client_name'>[];
	const names = await resolveEngagementClientNames(
		db,
		tenantId,
		rows.map((r) => r.engagement_id).filter((x): x is string => typeof x === 'string')
	);
	return rows.map((r) => ({
		...r,
		client_name: r.engagement_id ? (names.get(r.engagement_id) ?? null) : null
	}));
}
