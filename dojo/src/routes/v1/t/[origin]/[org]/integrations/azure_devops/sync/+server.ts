// POST /v1/t/{origin}/{org}/integrations/azure_devops/sync — re-read one of this
// dōjō's Azure DevOps organizations with its STORED grant (#308 R5b, D42):
// `{ organization_id }`. The refresh token is refreshed (and the rotated one
// stored); the admin re-proved, the projects filed, the members followed. A grant
// Entra refuses marks the connection "needs a new token" (409 reconnect). The
// body carries no token. ADMIN floor; 503 when the app registration is not
// configured.
import { dojoDb } from '$lib/server/dojo-supabase';
import { organizationRequest, syncAzureOrganization } from '$lib/server/azure-import';
import { azureConfig, azureRoute } from '$lib/server/azure-route';
import { azureAdapter } from '$lib/server/forge-azure';

export const POST = azureRoute(async ({ caller, event }) => {
	const { orgId } = await organizationRequest(event.request);
	const cfg = azureConfig();
	return Response.json(
		await syncAzureOrganization(dojoDb(), caller.userId, caller.tenantId, orgId, { cfg, adapterFor: (org) => azureAdapter(org) })
	);
});
