// GET/POST /v1/t/{origin}/{org}/billing — the tenant's per-seat billing.
//   GET  → { account, usage } — the billing account snapshot plus LIVE seat
//          usage (unique users on private workspaces, with the per-user breakdown
//          that makes the count defensible). Read-only.
//   POST → recompute the live count and persist it into billing_accounts
//          (seats_used + seats_computed_at), returning { seats_used }.
// Billing is org-financial data → admin-gated. Per-seat model + the seat table
// are D-BILLING (schema + route only); no payment provider is wired.
//
// GET reads the seats and the account as the signed-in admin (their policies
// decide); the seats' workspace names are a system lookup, and so are the seat
// holders' names — `resolveDisplayNames` through dojoDb(), the access matrix's
// exception (callers cannot read `principals`). A failed name read is an error,
// never a 200 of null names (D83). POST is the system's:
// seats_used is a money-facing cache derived from the seats, never a number a
// session writes (#236, access matrix "What stays service_role").
import type { RequestHandler } from './$types';
import { dojoDb } from '$lib/server/dojo-supabase';
import { callerDb } from '$lib/server/caller-db';
import { resolveTenantAccess, apiError, ACCESS } from '$lib/server/dojo-auth';
import { resolveDisplayNames } from '$lib/server/identity-resolve';
import { AdminError } from '$lib/server/admin-data';
import { nameSeatHolders } from '$lib/billing-view';
import {
	loadActiveSeatRows,
	summarizeSeatUsage,
	getBillingAccount,
	refreshSeatsUsed,
	BillingError
} from '$lib/server/billing-data';

export const GET: RequestHandler = async ({ params, request, locals }) => {
	try {
		const caller = await resolveTenantAccess(
			params.origin,
			params.org,
			request,
			locals,
			ACCESS.admin
		);
		const { tenantId } = caller;
		const db = callerDb(caller);
		const count = summarizeSeatUsage(await loadActiveSeatRows(db, tenantId, dojoDb()));
		const ids = count.billable_users.map((u) => u.user_id);
		const names = ids.length > 0 ? await resolveDisplayNames(dojoDb(), tenantId, ids) : new Map();
		const usage = nameSeatHolders(count, names);
		const account = await getBillingAccount(db, tenantId);
		return Response.json({ account, usage });
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof BillingError || e instanceof AdminError) return apiError(e.status, e.message);
		throw e;
	}
};

export const POST: RequestHandler = async ({ params, request, locals }) => {
	try {
		const { tenantId } = await resolveTenantAccess(
			params.origin,
			params.org,
			request,
			locals,
			ACCESS.admin
		);
		const db = dojoDb();
		const usage = summarizeSeatUsage(await loadActiveSeatRows(db, tenantId, db));
		await refreshSeatsUsed(db, tenantId, usage.seats_used, new Date().toISOString());
		return Response.json({ seats_used: usage.seats_used });
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof BillingError) return apiError(e.status, e.message);
		throw e;
	}
};
