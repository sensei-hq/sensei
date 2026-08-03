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
	'id, user_id, provider, subject, email, display_name, created_at, last_login_at';
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

	// relay_* tables key on membership_id (WS-0 Rule A) — scope the tenant's live
	// sessions via its memberships, not a dropped tenant_id column.
	const mem = await db.from('memberships').select('id').eq('tenant_id', tenantId);
	if (mem.error) throw new AdminError(500, mem.error.message);
	const membershipIds = (mem.data ?? []).map((m) => m.id as string);

	// connections — live relay sessions (heartbeat within 5 min).
	const conn = await db
		.from('relay_sessions')
		.select('id', { count: 'exact', head: true })
		.in('membership_id', membershipIds)
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

/** The membership kinds — `dojo.membership_kind`. */
export const MEMBERSHIP_KINDS = ['employer', 'client', 'community', 'personal'] as const;

/**
 * Validate an untrusted `POST …/members` body into a {@link NewMemberInput}, or
 * throw AdminError(400). `role` defaults to `contributor` when neither `role` nor
 * `git_provider_role` resolves to a known role (dojo-mind derives from the
 * `dojo.roles` map; the console always sends an explicit role for the override).
 */
export function parseNewMember(body: Record<string, unknown>): NewMemberInput {
	const userId = typeof body.user_id === 'string' ? body.user_id.trim() : '';
	if (!userId) throw new AdminError(400, 'user_id is required');
	const kind = typeof body.kind === 'string' ? body.kind : '';
	if (!(MEMBERSHIP_KINDS as readonly string[]).includes(kind)) {
		throw new AdminError(400, 'kind must be employer, client, community, or personal');
	}
	const via = body.authenticated_via;
	if (!isAuthMethod(via)) {
		throw new AdminError(400, 'authenticated_via must be sso, github_oauth, or device_code');
	}
	// Explicit `role` wins; else the git-provider-derived role; else contributor.
	const role = isMemberRole(body.role)
		? body.role
		: isMemberRole(body.git_provider_role)
			? body.git_provider_role
			: 'contributor';
	const out: NewMemberInput = { user_id: userId, kind, authenticated_via: via as string, role };
	if (isAttributionMode(body.attribution_default)) {
		out.attribution_default = body.attribution_default as string;
	}
	return out;
}

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
 * Create a dōjō the caller owns (F3a). The tenant is a custom-registered `org`
 * origin with key `org/{slug}`; the creator becomes its `admin`. A key collision
 * (someone already registered that name) is a 409. The creator's admin membership
 * reuses {@link addMember} (its own duplicate guard). Fails closed.
 */
export async function createDojo(
	db: DojoClient,
	userId: string,
	input: NewDojoInput
): Promise<{ id: string; key: string; name: string }> {
	const slug = slugify(input.name);
	const key = `org/${slug}`;
	const { data, error } = await db
		.from('tenants')
		.insert({
			key,
			origin: 'org',
			org: slug,
			name: input.name,
			dojo_url: `dojo.sensei-hq.org/${key}`,
			scope: 'private'
		})
		.select('id, key, name')
		.single();
	if (error) {
		if (error.code === '23505') throw new AdminError(409, 'a dōjō with that name already exists');
		throw new AdminError(500, error.message);
	}
	const tenant = data as { id: string; key: string; name: string };
	// The creator becomes admin — reuse addMember (shares the membership insert +
	// its own duplicate guard). authenticated_via = `sso` (the web/JWT console
	// auth plane; `dojo.auth_method` is sso | github_oauth | device_code).
	await addMember(db, tenant.id, {
		user_id: userId,
		kind: input.kind,
		authenticated_via: 'sso',
		role: 'admin'
	});
	return tenant;
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

// ── identities: create / patch / delete ──────────────────────────────────────

/** A validated `POST …/identities` body — the port of dojo-mind's `NewIdentity`. */
export interface NewIdentityInput {
	user_id: string;
	provider: string;
	subject: string;
	email?: string;
	display_name?: string;
}

/** Validate a `POST …/identities` body into a {@link NewIdentityInput}, or throw
 *  AdminError(400). `user_id`, `provider` (an auth method), `subject` required. */
export function parseNewIdentity(body: Record<string, unknown>): NewIdentityInput {
	const userId = typeof body.user_id === 'string' ? body.user_id.trim() : '';
	if (!userId) throw new AdminError(400, 'user_id is required');
	if (!isAuthMethod(body.provider)) {
		throw new AdminError(400, 'provider must be sso, github_oauth, or device_code');
	}
	const subject = typeof body.subject === 'string' ? body.subject.trim() : '';
	if (!subject) throw new AdminError(400, 'subject is required');
	const out: NewIdentityInput = { user_id: userId, provider: body.provider as string, subject };
	if (typeof body.email === 'string') out.email = body.email;
	if (typeof body.display_name === 'string') out.display_name = body.display_name;
	return out;
}

/**
 * Wire an identity provider (`POST …/identities`) — the port of dojo-mind's
 * `add_identity`. Returns `{ id }`; a duplicate `(tenant, provider, subject)` is
 * a 409.
 */
export async function createIdentity(
	db: DojoClient,
	tenantId: string,
	input: NewIdentityInput
): Promise<{ id: string }> {
	const { data, error } = await db
		.from('identities')
		.insert({
			tenant_id: tenantId,
			user_id: input.user_id,
			provider: input.provider,
			subject: input.subject,
			email: input.email ?? null,
			display_name: input.display_name ?? null
		})
		.select('id')
		.single();
	if (error) {
		if (error.code === '23505') throw new AdminError(409, 'that identity is already mapped');
		throw new AdminError(500, error.message);
	}
	return data as { id: string };
}

/** A validated `PATCH …/identities/{id}` body — email + display_name only. */
export interface PatchIdentityInput {
	email?: string;
	display_name?: string;
}

/** Validate a `PATCH …/identities/{id}` body into a {@link PatchIdentityInput}. */
export function parsePatchIdentity(body: Record<string, unknown>): PatchIdentityInput {
	const out: PatchIdentityInput = {};
	if (body.email !== undefined) {
		if (typeof body.email !== 'string') throw new AdminError(400, 'email must be a string');
		out.email = body.email;
	}
	if (body.display_name !== undefined) {
		if (typeof body.display_name !== 'string') {
			throw new AdminError(400, 'display_name must be a string');
		}
		out.display_name = body.display_name;
	}
	return out;
}

/**
 * Update an identity's email / display name (`PATCH …/identities/{id}`) — the
 * port of dojo-mind's `update_identity`. Returns `{ id }`; a 404 when none matches.
 */
export async function updateIdentity(
	db: DojoClient,
	tenantId: string,
	id: string,
	input: PatchIdentityInput
): Promise<{ id: string }> {
	const row: Record<string, unknown> = {};
	if (input.email !== undefined) row.email = input.email;
	if (input.display_name !== undefined) row.display_name = input.display_name;
	const { data, error } = await db
		.from('identities')
		.update(row)
		.eq('tenant_id', tenantId)
		.eq('id', id)
		.select('id')
		.maybeSingle();
	if (error) throw new AdminError(500, error.message);
	if (!data) throw new AdminError(404, 'no such identity');
	return data as { id: string };
}

/**
 * Delete an identity by id (`DELETE …/identities/{id}`) — returns `true` when a
 * row was removed, `false` when none matched (the handler maps false → 404).
 */
export async function deleteIdentity(db: DojoClient, tenantId: string, id: string): Promise<boolean> {
	const { data, error } = await db
		.from('identities')
		.delete()
		.eq('tenant_id', tenantId)
		.eq('id', id)
		.select('id');
	if (error) throw new AdminError(500, error.message);
	return (data?.length ?? 0) > 0;
}
