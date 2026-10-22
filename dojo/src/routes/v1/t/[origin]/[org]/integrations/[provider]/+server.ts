// POST /v1/t/{origin}/{org}/integrations/{provider} — Admin → Integrations
// (#308 R5a): the units (GitHub orgs) the body's token can see, each with what it
// is today and what this dōjō's admin may do with it. POST because the token
// travels in the body, never in a URL. ADMIN floor; the reads are system work
// (every tenant's connections), named. Fails closed.
import { ACCESS } from '$lib/server/dojo-auth';
import { dojoDb } from '$lib/server/dojo-supabase';
import { forgeAdapter } from '$lib/server/forge-adapter';
import { listIntegrationUnits, listRequest, providerParam } from '$lib/server/integrations-data';
import { tenantRoute } from '$lib/server/tenant-route';

export const POST = tenantRoute(ACCESS.admin, async (_db, caller, { params, request }) => {
	const adapter = forgeAdapter(providerParam(params.provider));
	const { token } = await listRequest(request);
	return Response.json(await listIntegrationUnits(dojoDb(), caller.userId, caller.tenantId, token, adapter));
});
