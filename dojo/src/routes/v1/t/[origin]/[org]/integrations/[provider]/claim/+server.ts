// POST /v1/t/{origin}/{org}/integrations/{provider}/claim — claim a unit's dōjō
// (#308 R5a). The body's token proves, now, that the caller administers the
// unit; its dōjō is made if it has none, and claimed (first claim wins, audited).
// ADMIN floor of the dōjō whose screen asked; the write is system work.
import { ACCESS } from '$lib/server/dojo-auth';
import { dojoDb } from '$lib/server/dojo-supabase';
import { forgeAdapter } from '$lib/server/forge-adapter';
import { claimIntegrationUnit, claimRequest, providerParam } from '$lib/server/integrations-data';
import { tenantRoute } from '$lib/server/tenant-route';

export const POST = tenantRoute(ACCESS.admin, async (_db, caller, { params, request }) => {
	const adapter = forgeAdapter(providerParam(params.provider));
	const { token, unitId } = await claimRequest(request);
	return Response.json(await claimIntegrationUnit(dojoDb(), caller.userId, token, adapter, unitId));
});
