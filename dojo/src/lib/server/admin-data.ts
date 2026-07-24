// Pure/injectable data logic for the Worker admin-console read endpoints
// (`GET …/members` · `…/policies` · `…/identities` · `…/audit` · `…/health`) —
// the TS port of dojo-mind's admin store reads over `dojo.memberships` /
// `policies` / `identities` / `audit_events` / (`relay_sessions` + memberships
// for the health rollup). Read-only this chunk (set-role / upsert-policy are
// follow-ups). Kept out of `+server.ts` so the query + rollup shaping are
// unit-testable without a Worker.
//
// Wire contract: matches the console client `admin-data.ts` (the shipped
// `(console)` admin screens) — `Membership[]` / `Policy[]` / `Identity[]` /
// `AuditEvent[]` / `HealthRollup`, wrapped in the same envelopes the client
// unwraps (`{ members }` / `{ policies }` / `{ identities }` / `{ events }`; the
// health rollup is bare).

import type { dojoDb } from './dojo-supabase';

/** The supabase-js client returned by `dojoDb()` (scoped to the `dojo` schema). */
export type DojoClient = ReturnType<typeof dojoDb>;

/** A domain error carrying the HTTP status the handler should return. */
export class AdminError extends Error {
	constructor(
		readonly status: number,
		message: string
	) {
		super(message);
	}
}

// ── wire row shapes (mirror the console client types) ────────────────────────

/** One membership row — the console `Membership`. */
export interface Membership {
	id: string;
	user_id: string;
	role: string;
	kind: string;
	authenticated_via: string;
	sync_status: string;
	attribution_default: string;
	last_heartbeat_at: string | null;
	disabled_at: string | null;
	created_at: string;
}

/** One identity mapping — the console `Identity`. */
export interface Identity {
	id: string;
	user_id: string;
	provider: string;
	subject: string;
	email: string | null;
	display_name: string | null;
	created_at: string;
	last_login_at: string | null;
}

/** One policy row — the console `Policy`. */
export interface Policy {
	id: string;
	scope_key: string;
	attribution_default: string;
	confidentiality: unknown;
	retention_days: number | null;
	created_at: string;
	updated_at: string;
}

/** One audit event — the console `AuditEvent`. */
export interface AuditEvent {
	id: number;
	ts: string;
	actor_id: string | null;
	engagement_id: string | null;
	action: string;
	target: string | null;
	detail: unknown;
}

/** The health strip — the console `HealthRollup`. */
export interface HealthRollup {
	connections: number;
	queue_depth: number;
	publish_rate_1h: number;
	error_rate_1h: number;
}

const MEMBER_COLS =
	'id, user_id, role, kind, authenticated_via, sync_status, attribution_default, last_heartbeat_at, disabled_at, created_at';
const IDENTITY_COLS =
	'id, user_id, provider, subject, email, display_name, created_at, last_login_at';
const POLICY_COLS =
	'id, scope_key, attribution_default, confidentiality, retention_days, created_at, updated_at';
const AUDIT_COLS = 'id, ts, actor_id, engagement_id, action, target, detail';

/** List the tenant's memberships, most recent first. */
export async function listMembers(db: DojoClient, tenantId: string): Promise<Membership[]> {
	const { data, error } = await db
		.from('memberships')
		.select(MEMBER_COLS)
		.eq('tenant_id', tenantId)
		.order('created_at', { ascending: false });
	if (error) throw new AdminError(500, error.message);
	return (data ?? []) as unknown as Membership[];
}

/** List the tenant's identity mappings, most recent first. */
export async function listIdentities(db: DojoClient, tenantId: string): Promise<Identity[]> {
	const { data, error } = await db
		.from('identities')
		.select(IDENTITY_COLS)
		.eq('tenant_id', tenantId)
		.order('created_at', { ascending: false });
	if (error) throw new AdminError(500, error.message);
	return (data ?? []) as unknown as Identity[];
}

/** List the tenant's policy grid, by scope key. */
export async function listPolicies(db: DojoClient, tenantId: string): Promise<Policy[]> {
	const { data, error } = await db
		.from('policies')
		.select(POLICY_COLS)
		.eq('tenant_id', tenantId)
		.order('scope_key', { ascending: true });
	if (error) throw new AdminError(500, error.message);
	return (data ?? []) as unknown as Policy[];
}

/** List the tenant's audit events, most recent first, capped at `limit`
 *  (default 100, clamped to 1..500). */
export async function listAudit(
	db: DojoClient,
	tenantId: string,
	limit = 100
): Promise<AuditEvent[]> {
	const capped = Math.max(1, Math.min(500, Number.isFinite(limit) ? limit : 100));
	const { data, error } = await db
		.from('audit_events')
		.select(AUDIT_COLS)
		.eq('tenant_id', tenantId)
		.order('ts', { ascending: false })
		.limit(capped);
	if (error) throw new AdminError(500, error.message);
	return (data ?? []) as unknown as AuditEvent[];
}

/**
 * The admin health rollup over `dojo.relay_sessions` (heartbeat/liveness) +
 * `dojo.memberships` (sync_status) + `dojo.triage_queue` (queue depth) — the
 * port of dojo-mind's `health_rollup`, adapted to the tables the Worker owns:
 *   • connections   = relay sessions with a heartbeat in the last 5 min (live).
 *   • queue_depth   = open (`queued`) triage rows.
 *   • publish_rate_1h = audit events with a publish-ish action in the last hour.
 *   • error_rate_1h = memberships in `sync_status = 'error'` (the shared-mind
 *     error signal the Worker has; dojo-mind counted error audit events, which
 *     the tenant audit trail doesn't yet distinguish — a follow-up).
 *
 * Each count is an isolated `head`/`count` query so a single missing table can't
 * blank the whole strip; a query error throws (never a silent 0).
 */
export async function getHealth(db: DojoClient, tenantId: string): Promise<HealthRollup> {
	const now = Date.now();
	const fiveMinAgo = new Date(now - 5 * 60_000).toISOString();
	const oneHourAgo = new Date(now - 60 * 60_000).toISOString();

	// connections — live relay sessions (heartbeat within 5 min).
	const conn = await db
		.from('relay_sessions')
		.select('id', { count: 'exact', head: true })
		.eq('tenant_id', tenantId)
		.gte('heartbeat_at', fiveMinAgo);
	if (conn.error) throw new AdminError(500, conn.error.message);

	// queue_depth — open triage rows.
	const queue = await db
		.from('triage_queue')
		.select('id', { count: 'exact', head: true })
		.eq('tenant_id', tenantId)
		.eq('state', 'queued');
	if (queue.error) throw new AdminError(500, queue.error.message);

	// publish_rate_1h — publish/approve/distribute audit events in the last hour.
	const publish = await db
		.from('audit_events')
		.select('id', { count: 'exact', head: true })
		.eq('tenant_id', tenantId)
		.in('action', ['publish', 'approve', 'distribute'])
		.gte('ts', oneHourAgo);
	if (publish.error) throw new AdminError(500, publish.error.message);

	// error_rate_1h — memberships whose device sync is erroring.
	const errors = await db
		.from('memberships')
		.select('id', { count: 'exact', head: true })
		.eq('tenant_id', tenantId)
		.eq('sync_status', 'error');
	if (errors.error) throw new AdminError(500, errors.error.message);

	return {
		connections: conn.count ?? 0,
		queue_depth: queue.count ?? 0,
		publish_rate_1h: publish.count ?? 0,
		error_rate_1h: errors.count ?? 0
	};
}
