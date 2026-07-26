// GET/POST /v1/t/{origin}/{org}/billing — the tenant's per-seat billing.
//   GET  → { account, usage } — the billing account snapshot plus LIVE seat
//          usage (unique users on private projects, with the per-user breakdown
//          that makes the count defensible). Read-only.
//   POST → recompute the live count and persist it into billing_accounts
//          (seats_used + seats_computed_at), returning { seats_used }.
// Billing is org-financial data → admin-gated. Per-seat model + the seat table
// are D-BILLING (schema + route only); no payment provider is wired.
import type { RequestHandler } from './$types';
import { dojoDb } from '$lib/server/dojo-supabase';
import { resolveTenantAccess, apiError, ACCESS } from '$lib/server/dojo-auth';
import {
	loadActiveSeatRows,
	summarizeSeatUsage,
	getBillingAccount,
	refreshSeatsUsed,
	BillingError
} from '$lib/server/billing-data';

export const GET: RequestHandler = async ({ params, request, locals }) => {
	try {
		const { tenantId } = await resolveTenantAccess(
			params.origin,
			params.org,
			request,
			locals,
			ACCESS.admin
		);
		const db = dojoDb();
		const usage = summarizeSeatUsage(await loadActiveSeatRows(db, tenantId));
		const account = await getBillingAccount(db, tenantId);
		return Response.json({ account, usage });
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof BillingError) return apiError(e.status, e.message);
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
		const usage = summarizeSeatUsage(await loadActiveSeatRows(db, tenantId));
		await refreshSeatsUsed(db, tenantId, usage.seats_used, new Date().toISOString());
		return Response.json({ seats_used: usage.seats_used });
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof BillingError) return apiError(e.status, e.message);
		throw e;
	}
};
