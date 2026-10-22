// POST /v1/t/{origin}/{org}/integrations/{provider}/claim — claim a unit's dōjō
// (#308 R5a). The connected GitHub token proves, now, that the caller administers
// the unit; its dōjō is made if it has none, and claimed (first claim wins,
// audited). ADMIN floor of the dōjō whose screen asked; the write is system work.
import { dojoDb } from '$lib/server/dojo-supabase';
import { claimIntegrationUnit, claimRequest } from '$lib/server/integrations-data';
import { integrationsRoute } from '$lib/server/integrations-route';

export const POST = integrationsRoute(async ({ caller, adapter, token, request }) => {
	const { unitId } = await claimRequest(request);
	return Response.json(await claimIntegrationUnit(dojoDb(), caller.userId, token, adapter, unitId));
});
