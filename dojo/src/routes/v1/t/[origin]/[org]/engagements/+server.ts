// GET/POST /v1/t/{origin}/{org}/engagements — the tenant's client engagements.
// The in-Worker port of dojo-mind's `list_engagements` / `create_engagement`
// (api.rs), lead+ gated. This is the reference pattern for the other console
// resources (incidents, members, identities, policies, triage).
import type { RequestHandler } from './$types';
import { dojoDb } from '$lib/server/dojo-supabase';
import { resolveTenantAccess, apiError, ACCESS } from '$lib/server/dojo-auth';
import { countEngagementArtifacts } from '$lib/server/engagement-artifact-counts';
import { AdminError } from '$lib/server/admin-data';

const COLS =
	'id, client_tenant_id, client_name, description, project_bindings, policy_overrides, status, starts_on, ends_on, created_at, updated_at';

export const GET: RequestHandler = async ({ params, request, locals }) => {
	try {
		const { tenantId } = await resolveTenantAccess(params.origin, params.org, request, locals, ACCESS.lead);
		const db = dojoDb();
		const { data, error } = await db
			.from('engagements')
			.select(COLS)
			.eq('tenant_id', tenantId)
			.order('created_at', { ascending: false });
		if (error) return apiError(500, error.message);
		const rows = (data ?? []) as { id: string }[];
		// Enrich each row with its real kept/stripped artifact counts (was hardcoded 0
		// in the mapper): published → lessons_kept, archived → stripped.
		const counts = await countEngagementArtifacts(
			db,
			tenantId,
			rows.map((r) => r.id)
		);
		const engagements = rows.map((r) => {
			const c = counts.get(r.id);
			return { ...r, lessons_kept: c?.lessonsKept ?? 0, stripped: c?.stripped ?? 0 };
		});
		return Response.json({ engagements });
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof AdminError) return apiError(e.status, e.message);
		throw e;
	}
};

export const POST: RequestHandler = async ({ params, request, locals }) => {
	try {
		const { tenantId } = await resolveTenantAccess(params.origin, params.org, request, locals, ACCESS.lead);
		const body = (await request.json().catch(() => ({}))) as Record<string, unknown>;
		// Rule C: `client` split into `client_name` (required display name) +
		// `client_tenant_id` (optional FK to the client's own dojo.tenants row).
		const clientName = typeof body.client_name === 'string' ? body.client_name.trim() : '';
		if (!clientName) return apiError(400, 'client_name is required');
		const clientTenantId = typeof body.client_tenant_id === 'string' ? body.client_tenant_id : null;
		const { data, error } = await dojoDb()
			.from('engagements')
			.insert({
				tenant_id: tenantId,
				client_tenant_id: clientTenantId,
				client_name: clientName,
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
