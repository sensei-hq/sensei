// POST /v1/t/{origin}/{org}/integrations/{provider}/map — say where a unit's
// repositories file (#308 R5a): `{ as: 'dojo' }` (its own dōjō; a proven admin of
// the unit only) or `{ as: 'workspace', name? }` (a workspace of THIS dōjō). The
// connected GitHub token is read now; dojo.map_forge_unit enforces who may, and
// its refusal keeps its status. ADMIN floor; the write is system work.
import { dojoDb } from '$lib/server/dojo-supabase';
import { mapIntegrationUnit, mapRequest } from '$lib/server/integrations-data';
import { integrationsRoute } from '$lib/server/integrations-route';

export const POST = integrationsRoute(async ({ caller, adapter, token, request }) => {
	const { unitId, target } = await mapRequest(request);
	return Response.json(
		await mapIntegrationUnit(dojoDb(), caller.userId, caller.tenantId, token, adapter, unitId, target)
	);
});
