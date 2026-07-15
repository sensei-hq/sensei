// GET/POST /v1/t/{origin}/{org}/engagements — the tenant's client engagements.
// The in-Worker port of dojo-mind's `list_engagements` / `create_engagement`
// (api.rs), lead+ gated. This is the reference pattern for the other console
// resources (incidents, members, identities, policies, triage).
import type { RequestHandler } from './$types';
import { dojoDb } from '$lib/server/dojo-supabase';
import { resolveTenantAccess, apiError, ACCESS } from '$lib/server/dojo-auth';

const COLS =
	'id, client, description, project_bindings, policy_overrides, status, starts_on, ends_on, created_at, updated_at';

export const GET: RequestHandler = async ({ params, request, locals }) => {
	try {
		const { tenantId } = await resolveTenantAccess(params.origin, params.org, request, locals, ACCESS.lead);
		const { data, error } = await dojoDb()
			.from('engagements')
			.select(COLS)
			.eq('tenant_id', tenantId)
			.order('created_at', { ascending: false });
		if (error) return apiError(500, error.message);
		return Response.json({ engagements: data ?? [] });
	} catch (e) {
		if (e instanceof Response) return e;
		throw e;
	}
};

export const POST: RequestHandler = async ({ params, request, locals }) => {
	try {
		const { tenantId } = await resolveTenantAccess(params.origin, params.org, request, locals, ACCESS.lead);
		const body = (await request.json().catch(() => ({}))) as Record<string, unknown>;
		const client = typeof body.client === 'string' ? body.client.trim() : '';
		if (!client) return apiError(400, 'client is required');
		const { data, error } = await dojoDb()
			.from('engagements')
			.insert({
				tenant_id: tenantId,
				client,
				description: typeof body.description === 'string' ? body.description : null,
				project_bindings: body.project_bindings ?? [],
				policy_overrides: body.policy_overrides ?? {}
			})
			.select('id')
			.single();
		if (error) return apiError(500, error.message);
		return Response.json({ id: data.id });
	} catch (e) {
		if (e instanceof Response) return e;
		throw e;
	}
};
