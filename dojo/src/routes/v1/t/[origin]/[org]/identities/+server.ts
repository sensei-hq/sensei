// GET /v1/t/{origin}/{org}/identities — the identities of the tenant's members,
// for the admin console, on the JWT plane at the ADMIN floor. Returns
// `{ identities: Identity[] }`.
//
// READ ONLY. An identity is the person's proof (provider, subject) and spans
// every tenant they belong to, so no tenant's admin may write one: identities
// are created by provisioning, with the forge's proof (#236, access matrix
// "Tenancy, identity, billing").
import type { RequestHandler } from './$types';
import { dojoDb } from '$lib/server/dojo-supabase';
import { resolveTenantAccess, apiError, ACCESS } from '$lib/server/dojo-auth';
import { listIdentities, AdminError } from '$lib/server/admin-data';

export const GET: RequestHandler = async ({ params, request, locals }) => {
	try {
		const { tenantId } = await resolveTenantAccess(
			params.origin,
			params.org,
			request,
			locals,
			ACCESS.admin
		);
		const identities = await listIdentities(dojoDb(), tenantId);
		return Response.json({ identities });
	} catch (e) {
		if (e instanceof Response) return e;
		if (e instanceof AdminError) return apiError(e.status, e.message);
		throw e;
	}
};
