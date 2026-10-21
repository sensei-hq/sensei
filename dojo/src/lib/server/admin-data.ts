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
import { resolveDisplayNames } from './identity-resolve';
import { refusalStatus } from './db-refusal';
import { MEMBERSHIP_KINDS } from '../admin-view';

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
	/** WS-1: best display name resolved from `dojo.identities` (null → the caller
	 *  falls back to a shortId; never a fabricated name). */
	display_name: string | null;
	email: string | null;
}

/** One identity mapping — the console `Identity`. Keys on `principal_id`, not a
 *  login id: an identity says "this GitHub account is this person", and the
 *  person is a `dojo.principals` row (see that table, and §VIII.2). */
export interface Identity {
	id: string;
	principal_id: string;
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

// ── enum guards (mirror dojo-mind's `is_member_role` / `is_auth_method` /
//    `is_attribution_mode`; the DB enums would reject a bad literal, but we
//    validate first for a clean 400 rather than a 500). ──────────────────────
const MEMBER_ROLES = ['contributor', 'maintainer', 'lead', 'admin'] as const;
const AUTH_METHODS = ['sso', 'github_oauth', 'device_code'] as const;
const ATTRIBUTION_MODES = ['named', 'anonymous'] as const;

/** A `dojo.member_role`. */
export type MemberRole = (typeof MEMBER_ROLES)[number];

export function isMemberRole(v: unknown): v is MemberRole {
	return typeof v === 'string' && (MEMBER_ROLES as readonly string[]).includes(v);
}
function isAuthMethod(v: unknown): boolean {
	return typeof v === 'string' && (AUTH_METHODS as readonly string[]).includes(v);
}
function isAttributionMode(v: unknown): boolean {
	return typeof v === 'string' && (ATTRIBUTION_MODES as readonly string[]).includes(v);
}

const MEMBER_COLS =
	'id, user_id, role, kind, authenticated_via, sync_status, attribution_default, last_heartbeat_at, disabled_at, created_at';
const IDENTITY_COLS =
	'id, principal_id, provider, subject, email, display_name, created_at, last_login_at';
const POLICY_COLS =
	'id, scope_key, attribution_default, confidentiality, retention_days, created_at, updated_at';
const AUDIT_COLS = 'id, ts, actor_id, engagement_id, action, target, detail';

/** List the tenant's memberships, most recent first, each enriched with the
 *  member's display name + email resolved from `dojo.identities` (WS-1). A member
 *  with no identity row resolves to null/null (the mapper falls back to a shortId). */
export async function listMembers(db: DojoClient, tenantId: string): Promise<Membership[]> {
	const { data, error } = await db
		.from('memberships')
		.select(MEMBER_COLS)
		.eq('tenant_id', tenantId)
		.order('created_at', { ascending: false });
	if (error) throw new AdminError(500, error.message);
	const rows = (data ?? []) as unknown as Membership[];
	const names = await resolveDisplayNames(
		db,
		tenantId,
		rows.map((r) => r.user_id)
	);
	return rows.map((r) => {
		const n = names.get(r.user_id);
		return { ...r, display_name: n?.display_name ?? null, email: n?.email ?? null };
	});
}

// ── identity scoping ─────────────────────────────────────────────────────────
// `dojo.identities` is GLOBAL: no tenant_id, keyed on principal_id. That is
// deliberate — "this GitHub account is this person" is not a per-tenant fact,
// and the old tenant-scoped unique made one sign-in fan out into one identity
// row per tenant (see the table's DDL).
//
// But the routes are still tenant-addressed. So the isolation the tenant_id
// filter used to provide incidentally is now an EXPLICIT membership check:
// an admin of one tenant reads only their members' identities.
//
// And nothing here WRITES an identity. Global means an edit by tenant A's
// admin would change the row tenant B relies on, so identities are created by
// provisioning with the forge's proof, never through a tenant's console (#236,
// access matrix "Tenancy, identity, billing").

/** The principal ids of a tenant's members — what scopes a tenant-addressed
 *  identity read. `memberships.user_id` holds a principal id (§VIII.2). */
async function tenantPrincipalIds(db: DojoClient, tenantId: string): Promise<string[]> {
	const { data, error } = await db.from('memberships').select('user_id').eq('tenant_id', tenantId);
	if (error) throw new AdminError(500, error.message);
	return (data ?? []).map((m) => (m as { user_id: string }).user_id);
}

/** The identity mappings of this tenant's members, most recent first. Returns []
 *  for a member-less tenant — genuinely empty, and it skips the second query
 *  rather than issuing `.in(col, [])`. */
export async function listIdentities(db: DojoClient, tenantId: string): Promise<Identity[]> {
	const principalIds = await tenantPrincipalIds(db, tenantId);
	if (principalIds.length === 0) return [];
	const { data, error } = await db
		.from('identities')
		.select(IDENTITY_COLS)
		.in('principal_id', principalIds)
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

	// connections — the tenant's live relay sessions (heartbeat within 5 min). A
	// session is its owner's content, so the admin gets a COUNT from the database,
	// never the rows (access matrix "Relay", #236); anyone else is refused, loudly.
	const conn = await db.rpc('tenant_live_session_count', { p_tenant: tenantId, p_since: fiveMinAgo });
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
		connections: conn.data as number,
		queue_depth: queue.count ?? 0,
		publish_rate_1h: publish.count ?? 0,
		error_rate_1h: errors.count ?? 0
	};
}

// ════════════════════════════════════════════════════════════════════════════
// Writes — the port of dojo-mind's admin mutations (set-role · provision-member ·
// policy upsert/patch/delete · identity create/patch/delete). Each validates its
// untrusted body into a typed struct (throwing AdminError 400) and returns the
// row-shape the console client `admin-data.ts` expects. The audit write is the
// caller's responsibility (the handler calls `recordAudit` after the mutation so
// a swallowed audit can't fail a committed write); these stay pure over `db`.
// ════════════════════════════════════════════════════════════════════════════

// ── members: set-role + provision ────────────────────────────────────────────

/**
 * Override a member's role (`PATCH …/members/{user_id}/role`) — the port of
 * dojo-mind's `set_member_role`. Validates the role against `dojo.member_role`
 * (400 on a bad value), updates the tenant's membership for `userId`, and returns
 * `{ user_id, role }` (the shape `admin-data.ts setMemberRole()` unwraps). A 404
 * is thrown when no membership matches (unknown user in this tenant).
 */
export async function setMemberRole(
	db: DojoClient,
	tenantId: string,
	userId: string,
	role: unknown
): Promise<{ user_id: string; role: string }> {
	if (!isMemberRole(role)) {
		throw new AdminError(400, 'role must be contributor, maintainer, lead, or admin');
	}
	const { data, error } = await db
		.from('memberships')
		.update({ role, updated_at: new Date().toISOString() })
		.eq('tenant_id', tenantId)
		.eq('user_id', userId)
		.select('user_id, role')
		.maybeSingle();
	if (error) throw new AdminError(500, error.message);
	if (!data) throw new AdminError(404, 'no such member in this tenant');
	return data as { user_id: string; role: string };
}

/** A validated `POST …/members` body — the port of dojo-mind's `NewMembership`. */
export interface NewMemberInput {
	user_id: string;
	kind: string;
	authenticated_via: string;
	role: MemberRole;
	attribution_default?: string;
}

/** The membership kinds — `dojo.membership_kind` (shared with the client invite form). */
export { MEMBERSHIP_KINDS };

/**
 * Provision a membership (`POST …/members`) — the port of dojo-mind's
 * `add_membership`. Returns `{ id, role }` (the shape `admin-data.ts addMember()`
 * unwraps). A duplicate `(tenant, user)` is a 409. (Rule C: the dōjō url is
 * derived from the membership's tenant — `dojo.tenants.dojo_url` — not stored per
 * membership, so no `dojo_url` argument.)
 */
export async function addMember(
	db: DojoClient,
	tenantId: string,
	input: NewMemberInput
): Promise<{ id: string; role: string }> {
	const { data, error } = await db
		.from('memberships')
		.insert({
			tenant_id: tenantId,
			user_id: input.user_id,
			kind: input.kind,
			authenticated_via: input.authenticated_via,
			role: input.role,
			...(input.attribution_default ? { attribution_default: input.attribution_default } : {})
		})
		.select('id, role')
		.single();
	if (error) {
		// Unique (tenant_id, user_id) violation → already a member.
		if (error.code === '23505') throw new AdminError(409, 'already a member of this tenant');
		throw new AdminError(500, error.message);
	}
	return data as { id: string; role: string };
}

// ── self-serve dōjō create (F3a) ─────────────────────────────────────────────

/** A validated `POST /v1/you/dojos` body — the self-serve create. */
export interface NewDojoInput {
	name: string;
	kind: string;
}

/** A URL-clean slug from a display name: lowercase, non-alphanumerics → single
 *  hyphens, trimmed. `"Acme Corp!" → "acme-corp"`. Pure. */
export function slugify(name: string): string {
	return name
		.toLowerCase()
		.trim()
		.replace(/[^a-z0-9]+/g, '-')
		.replace(/^-+|-+$/g, '');
}

/** Validate a create-dōjō body into a {@link NewDojoInput}, or throw
 *  AdminError(400). `name` must slugify to something non-empty; `kind` must be a
 *  `dojo.membership_kind`. */
export function parseNewDojo(body: Record<string, unknown>): NewDojoInput {
	const name = typeof body.name === 'string' ? body.name.trim() : '';
	if (!name) throw new AdminError(400, 'name is required');
	if (!slugify(name)) throw new AdminError(400, 'name must contain a letter or digit');
	const kind = typeof body.kind === 'string' ? body.kind : '';
	if (!(MEMBERSHIP_KINDS as readonly string[]).includes(kind)) {
		throw new AdminError(400, 'kind must be employer, client, community, or personal');
	}
	return { name, kind };
}

/**
 * Create a dōjō the caller owns (F3a, issue #117). The tenant is a `team` with
 * key `team/{slug}` — `organization/` is the forge-proven organisation's alone
 * (decision 2026-10-12) — and the creator becomes its `admin`. A key collision (someone already registered that name) is a 409.
 * dojo.create_dojo does both in one transaction, for the SESSION's principal,
 * and builds the key and url itself (#236). Fails closed.
 *
 * The key's first segment is the ORIGIN, not a forge — `dojo-auth.ts` resolves a
 * tenant by joining the two URL segments back into `{origin}/{slug}`, so the
 * prefix and `origin` must agree or the tenant is unreachable at its own URL.
 * That is also what keeps `personal/acme`, `organization/acme` and `team/acme`
 * apart under the single unique on `key` (spec §IV.7).
 *
 * This wrote `origin: 'org'`, `org: slug` and `key: org/{slug}` until 5cbe4d4d's
 * sibling commit: all three were retired by the phase-1 schema change, and the
 * insert failed outright with `column "org" of relation "tenants" does not
 * exist`. A mocked test asserted the rejected payload and stayed green — hence
 * the live-Postgres coverage in database/tests/dojo/create_dojo_insert.sql.
 */
export async function createDojo(
	db: DojoClient,
	input: NewDojoInput
): Promise<{ id: string; key: string; name: string }> {
	const { data, error } = await db
		.rpc('create_dojo', { p_name: input.name, p_slug: slugify(input.name), p_kind: input.kind })
		.single();
	if (error) {
		if (error.code === '23505') throw new AdminError(409, 'a dōjō with that name already exists');
		throw new AdminError(refusalStatus(error), error.message);
	}
	return data as { id: string; key: string; name: string };
}

// ── policies: upsert / patch / delete ────────────────────────────────────────

/** A validated `POST …/policies` body — the port of dojo-mind's `UpsertPolicy`. */
export interface UpsertPolicyInput {
	scope_key: string;
	attribution_default?: string;
	confidentiality?: unknown;
	retention_days?: number | null;
}

/** Validate a retention_days value (a non-negative integer, or null/absent). */
function parseRetention(v: unknown): number | null | undefined {
	if (v == null) return v === null ? null : undefined;
	if (typeof v !== 'number' || !Number.isInteger(v) || v < 0) {
		throw new AdminError(400, 'retention_days must be a non-negative integer');
	}
	return v;
}

/**
 * Validate a `POST …/policies` body into an {@link UpsertPolicyInput}, or throw
 * AdminError(400). `scope_key` is required; `attribution_default` (when present)
 * must be a `dojo.attribution_mode`.
 */
export function parseUpsertPolicy(body: Record<string, unknown>): UpsertPolicyInput {
	const scopeKey = typeof body.scope_key === 'string' ? body.scope_key.trim() : '';
	if (!scopeKey) throw new AdminError(400, 'scope_key is required');
	const out: UpsertPolicyInput = { scope_key: scopeKey };
	if (body.attribution_default !== undefined) {
		if (!isAttributionMode(body.attribution_default)) {
			throw new AdminError(400, 'attribution_default must be named or anonymous');
		}
		out.attribution_default = body.attribution_default as string;
	}
	if (body.confidentiality !== undefined) out.confidentiality = body.confidentiality;
	const retention = parseRetention(body.retention_days);
	if (retention !== undefined) out.retention_days = retention;
	return out;
}

/**
 * Create-or-edit a policy by `(tenant, scope_key)` (`POST …/policies`, upsert) —
 * the port of dojo-mind's `upsert_policy`. Returns `{ id, scope_key }` (the shape
 * `admin-data.ts upsertPolicy()` unwraps).
 */
export async function upsertPolicy(
	db: DojoClient,
	tenantId: string,
	input: UpsertPolicyInput
): Promise<{ id: string; scope_key: string }> {
	const row: Record<string, unknown> = {
		tenant_id: tenantId,
		scope_key: input.scope_key,
		updated_at: new Date().toISOString()
	};
	if (input.attribution_default !== undefined) row.attribution_default = input.attribution_default;
	if (input.confidentiality !== undefined) row.confidentiality = input.confidentiality;
	if (input.retention_days !== undefined) row.retention_days = input.retention_days;
	const { data, error } = await db
		.from('policies')
		.upsert(row, { onConflict: 'tenant_id,scope_key' })
		.select('id, scope_key')
		.single();
	if (error) throw new AdminError(500, error.message);
	return data as { id: string; scope_key: string };
}

/** A validated `PATCH …/policies/{id}` body — the port of dojo-mind's
 *  `PatchPolicy` (every field optional; an absent field is left unchanged). */
export interface PatchPolicyInput {
	attribution_default?: string;
	confidentiality?: unknown;
	retention_days?: number | null;
}

/** Validate a `PATCH …/policies/{id}` body into a {@link PatchPolicyInput}. */
export function parsePatchPolicy(body: Record<string, unknown>): PatchPolicyInput {
	const out: PatchPolicyInput = {};
	if (body.attribution_default !== undefined) {
		if (!isAttributionMode(body.attribution_default)) {
			throw new AdminError(400, 'attribution_default must be named or anonymous');
		}
		out.attribution_default = body.attribution_default as string;
	}
	if (body.confidentiality !== undefined) out.confidentiality = body.confidentiality;
	const retention = parseRetention(body.retention_days);
	if (retention !== undefined) out.retention_days = retention;
	return out;
}

/**
 * Patch a policy by id (`PATCH …/policies/{id}`) — the port of dojo-mind's
 * `patch_policy`. Returns `{ id }`; a 404 is thrown when no tenant policy matches.
 */
export async function patchPolicy(
	db: DojoClient,
	tenantId: string,
	id: string,
	input: PatchPolicyInput
): Promise<{ id: string }> {
	const row: Record<string, unknown> = { updated_at: new Date().toISOString() };
	if (input.attribution_default !== undefined) row.attribution_default = input.attribution_default;
	if (input.confidentiality !== undefined) row.confidentiality = input.confidentiality;
	if (input.retention_days !== undefined) row.retention_days = input.retention_days;
	const { data, error } = await db
		.from('policies')
		.update(row)
		.eq('tenant_id', tenantId)
		.eq('id', id)
		.select('id')
		.maybeSingle();
	if (error) throw new AdminError(500, error.message);
	if (!data) throw new AdminError(404, 'no such policy');
	return data as { id: string };
}

/**
 * Delete a policy by id (`DELETE …/policies/{id}`) — returns `true` when a row
 * was removed, `false` when none matched (the handler maps false → 404).
 */
export async function deletePolicy(db: DojoClient, tenantId: string, id: string): Promise<boolean> {
	const { data, error } = await db
		.from('policies')
		.delete()
		.eq('tenant_id', tenantId)
		.eq('id', id)
		.select('id');
	if (error) throw new AdminError(500, error.message);
	return (data?.length ?? 0) > 0;
}
