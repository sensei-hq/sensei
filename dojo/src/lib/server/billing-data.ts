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
 *  namespace (visibility + name). Cross-schema (dojo.seats → sensei.namespaces),
 *  so it joins in two queries rather than a PostgREST embed. Orphan seats (the
 *  namespace was deleted) are dropped. */
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
		.schema('sensei')
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
