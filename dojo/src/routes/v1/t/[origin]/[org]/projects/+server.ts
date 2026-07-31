// GET /v1/t/{origin}/{org}/projects — the org's projects (dojo.projects,
// tenant-scoped). The JWT-plane console read, LEAD-floor gated. Read-only (the
// daemon upserts rows via the service_role on relay runs). Replaces the
// `orgProjectsFor(slug)` fixture — fails closed, honest-empty until populated.
import type { RequestHandler } from './$types';
import { dojoDb } from '$lib/server/dojo-supabase';
import { resolveTenantAccess, apiError, ACCESS } from '$lib/server/dojo-auth';
import { listOrgProjects, AdminError } from '$lib/server/projects-data';

export const GET: RequestHandler = async ({ params, request, locals }) => {
	try {
		const { tenantId } = await resolveTenantAccess(params.origin, params.org, request, locals, ACCESS.lead);
		const projects = await listOrgProjects(dojoDb(), tenantId);
		return Response.json({ projects });
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof AdminError) return apiError(e.status, e.message);
		throw e;
	}
};
