// The one tenant-scoped audit writer shared by every Worker mutation route
// (members / policies / identities) — the port of dojo-mind's `record_audit`,
// written to `dojo.audit_events` (the non-repudiation trail the admin console
// reads). Every admin write stamps one row here.
//
// Kept in one place so the audit shape (tenant + actor + action + target +
// detail) can never drift across the mutation modules, and so a single unit test
// covers the write + the fire-and-forget contract. Rules federation keeps its own
// `recordRulesAudit` (device-token plane, publish|retract); this is the JWT-plane
// admin trail.

import type { dojoDb } from './dojo-supabase';

/** The supabase-js client returned by `dojoDb()` (scoped to the `dojo` schema). */
export type DojoClient = ReturnType<typeof dojoDb>;

/** The fields an audit row carries beyond the tenant + actor. */
export interface AuditWrite {
	action: string;
	target?: string | null;
	detail?: Record<string, unknown>;
}

/**
 * Append a tenant-scoped audit event for an admin mutation. Fire-and-forget
 * from the handler's perspective: a failed audit write must NOT fail an
 * already-committed mutation (mirrors dojo-mind's swallowed audit on the request
 * path), but the error is surfaced (logged), never silently dropped.
 */
export async function recordAudit(
	db: DojoClient,
	tenantId: string,
	actorId: string,
	write: AuditWrite
): Promise<void> {
	const { error } = await db.from('audit_events').insert({
		tenant_id: tenantId,
		actor_id: actorId,
		action: write.action,
		target: write.target ?? null,
		detail: write.detail ?? {}
	});
	if (error) {
		// Non-fatal (the mutation already committed), but never silent.
		console.error(`audit write failed (${write.action} ${write.target ?? ''}): ${error.message}`);
	}
}
