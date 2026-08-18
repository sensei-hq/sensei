// Billing data layer for the in-Worker dojo API. Per-seat model (D-BILLING =
// schema + route only): a tenant's billable seats are the UNIQUE users actively
// seated on its PRIVATE projects (dojo.seats × sensei.namespaces.visibility),
// deduped across projects — never everyone with GitHub access, and public-only
// users never count. The authoritative count also lives in the SQL view
// dojo.tenant_seat_usage; this layer computes the same figure PLUS the per-user
// breakdown that makes the count defensible, and refreshes the cached
// billing_accounts.seats_used snapshot.
import type { DojoClient } from './rules-data';
export type { DojoClient };

/** A billing operation failure — carries the HTTP status the route should map. */
export class BillingError extends Error {
	constructor(
		public status: number,
		message: string
	) {
		super(message);
		this.name = 'BillingError';
	}
}

/** One active seat joined to its project's visibility — the raw input to
 *  {@link summarizeSeatUsage}. */
export interface SeatRow {
	user_id: string;
	role: string;
	namespace_id: string;
	project_name: string;
	project_slug: string;
	visibility: 'private' | 'public';
}

/** A billable user and the private projects that seat them (the defensible
 *  breakdown behind the count). */
export interface BillableUser {
	user_id: string;
	projects: { name: string; slug: string; role: string }[];
}

/** The computed seat usage for a tenant. */
export interface SeatUsage {
	/** Unique users with ≥1 active seat on a private project — the billable count. */
	seats_used: number;
	/** All active seat rows considered (private + public), for transparency. */
	total_active_seats: number;
	/** Per-user breakdown of the billable seats (private projects only), sorted. */
	billable_users: BillableUser[];
}

/** The persisted billing account snapshot for a tenant. */
export interface BillingAccount {
	plan: string;
	status: string;
	seats_included: number;
	seats_used: number;
	seats_computed_at: string | null;
	period_start: string | null;
	period_end: string | null;
}

const ACCOUNT_COLS =
	'plan, status, seats_included, seats_used, seats_computed_at, period_start, period_end';

/** Compute a tenant's billable seat usage from its active seat rows. Only
 *  PRIVATE-project seats count; a user on several private projects is ONE seat
 *  (deduped on user_id); public-only users contribute nothing. Pure — the single
 *  source of truth for the route's count, mirroring the dojo.tenant_seat_usage
 *  view so the two never diverge. */
export function summarizeSeatUsage(rows: SeatRow[]): SeatUsage {
	const byUser = new Map<string, BillableUser['projects']>();
	for (const r of rows) {
		if (r.visibility !== 'private') continue; // public work never consumes a seat
		const projects = byUser.get(r.user_id) ?? [];
		projects.push({ name: r.project_name, slug: r.project_slug, role: r.role });
		byUser.set(r.user_id, projects);
	}
	const billable_users = [...byUser.entries()]
		.map(([user_id, projects]) => ({ user_id, projects }))
		.sort((a, b) => a.user_id.localeCompare(b.user_id));
	return { seats_used: billable_users.length, total_active_seats: rows.length, billable_users };
}

/** Load a tenant's ACTIVE seats (ended_at IS NULL) joined to their project
 *  namespace (visibility + name). Cross-schema (dojo.seats → dojo.namespaces, a
 *  dojo view over sensei.namespaces), so it joins in two queries rather than a
 *  PostgREST embed. Orphan seats (the namespace was deleted) are dropped. */
export async function loadActiveSeatRows(db: DojoClient, tenantId: string): Promise<SeatRow[]> {
	const { data: seats, error } = await db
		.from('seats')
		.select('user_id, role, namespace_id')
		.eq('tenant_id', tenantId)
		.is('ended_at', null);
	if (error) throw new BillingError(500, error.message);
	const rows = (seats ?? []) as { user_id: string; role: string; namespace_id: string }[];
	if (rows.length === 0) return [];

	const ids = [...new Set(rows.map((s) => s.namespace_id))];
	const { data: ns, error: nsErr } = await db
		.from('namespaces')
		.select('id, name, slug, visibility')
		.in('id', ids);
	if (nsErr) throw new BillingError(500, nsErr.message);
	const nsById = new Map(
		((ns ?? []) as { id: string; name: string; slug: string; visibility: 'private' | 'public' }[]).map(
			(n) => [n.id, n]
		)
	);

	return rows.flatMap((s) => {
		const n = nsById.get(s.namespace_id);
		if (!n) return []; // seat whose project namespace is gone — not billable
		return [
			{
				user_id: s.user_id,
				role: s.role,
				namespace_id: s.namespace_id,
				project_name: n.name,
				project_slug: n.slug,
				visibility: n.visibility
			}
		];
	});
}

/** Fetch a tenant's billing account, or null if it has none yet. */
export async function getBillingAccount(
	db: DojoClient,
	tenantId: string
): Promise<BillingAccount | null> {
	const { data, error } = await db
		.from('billing_accounts')
		.select(ACCOUNT_COLS)
		.eq('tenant_id', tenantId)
		.maybeSingle();
	if (error) throw new BillingError(500, error.message);
	return (data as BillingAccount | null) ?? null;
}

/** Resolve a project namespace slug to its `sensei.namespaces` id (scope=project),
 *  or null when unknown. The daemon federates the run's project by slug (its
 *  cross-DB stable identity); the seat binds to the resolved namespace id. */
export async function resolveProjectNamespaceId(
	db: DojoClient,
	slug: string
): Promise<string | null> {
	const { data, error } = await db
		.from('namespaces')
		.select('id')
		.eq('scope_key', 'project')
		.eq('slug', slug)
		.maybeSingle();
	if (error) throw new BillingError(500, error.message);
	return (data as { id: string } | null)?.id ?? null;
}

/** Open or refresh a user's seat on a project: if an ACTIVE seat exists (partial
 *  unique on user_id+namespace_id where ended_at is null), bump its
 *  `last_active_at`; otherwise open a fresh one. Idempotent — the natural call on
 *  each relay federation ("this user is actively using sensei on this project").
 *  Returns whether a new seat was opened. */
export async function openOrRefreshSeat(
	db: DojoClient,
	seat: { tenantId: string; userId: string; namespaceId: string; role?: string; nowIso: string }
): Promise<{ opened: boolean; id: string }> {
	const { data: existing, error } = await db
		.from('seats')
		.select('id')
		.eq('user_id', seat.userId)
		.eq('namespace_id', seat.namespaceId)
		.is('ended_at', null)
		.maybeSingle();
	if (error) throw new BillingError(500, error.message);
	if (existing) {
		const id = (existing as { id: string }).id;
		const { error: uErr } = await db.from('seats').update({ last_active_at: seat.nowIso }).eq('id', id);
		if (uErr) throw new BillingError(500, uErr.message);
		return { opened: false, id };
	}
	const { data: inserted, error: iErr } = await db
		.from('seats')
		.insert({
			tenant_id: seat.tenantId,
			user_id: seat.userId,
			namespace_id: seat.namespaceId,
			role: seat.role ?? 'contributor',
			last_active_at: seat.nowIso
		})
		.select('id')
		.single();
	if (iErr) throw new BillingError(500, iErr.message);
	return { opened: true, id: (inserted as { id: string }).id };
}

/** Close a user's active seat on a project (set ended_at — soft close, history
 *  retained). Called when a user is removed from a project. Returns whether a
 *  seat was closed (false if there was no active one). */
export async function closeSeat(
	db: DojoClient,
	seat: { userId: string; namespaceId: string; nowIso: string }
): Promise<{ closed: boolean }> {
	const { data, error } = await db
		.from('seats')
		.update({ ended_at: seat.nowIso })
		.eq('user_id', seat.userId)
		.eq('namespace_id', seat.namespaceId)
		.is('ended_at', null)
		.select('id');
	if (error) throw new BillingError(500, error.message);
	return { closed: (data as unknown[] | null ?? []).length > 0 };
}

/** Persist a freshly computed seat count into the tenant's billing account
 *  (upsert — creates a default `free`/`active` account on first refresh, else
 *  updates the cached snapshot). `nowIso` is injected for deterministic tests. */
export async function refreshSeatsUsed(
	db: DojoClient,
	tenantId: string,
	seatsUsed: number,
	nowIso: string
): Promise<void> {
	const { error } = await db
		.from('billing_accounts')
		.upsert(
			{ tenant_id: tenantId, seats_used: seatsUsed, seats_computed_at: nowIso },
			{ onConflict: 'tenant_id' }
		);
	if (error) throw new BillingError(500, error.message);
}
