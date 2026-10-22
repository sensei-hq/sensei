// POST /v1/t/{origin}/{org}/integrations/{provider}/map — say where a unit's
// repositories file (#308 R5a): `{ as: 'dojo' }` (its own dōjō; a proven admin of
// the unit only) or `{ as: 'workspace', name? }` (a workspace of THIS dōjō).
// The body's token is read now; dojo.map_forge_unit enforces who may, and its
// refusal keeps its status. ADMIN floor; the write is system work.
import { ACCESS } from '$lib/server/dojo-auth';
import { dojoDb } from '$lib/server/dojo-supabase';
import { forgeAdapter } from '$lib/server/forge-adapter';
import { mapIntegrationUnit, mapRequest, providerParam } from '$lib/server/integrations-data';
import { tenantRoute } from '$lib/server/tenant-route';

export const POST = tenantRoute(ACCESS.admin, async (_db, caller, { params, request }) => {
	const adapter = forgeAdapter(providerParam(params.provider));
	const { token, unitId, target } = await mapRequest(request);
	return Response.json(
		await mapIntegrationUnit(dojoDb(), caller.userId, caller.tenantId, token, adapter, unitId, target)
	);
});
