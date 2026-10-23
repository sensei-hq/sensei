// POST /v1/t/{origin}/{org}/integrations/azure_devops/sync — re-read one of this
// dōjō's Azure DevOps organizations with its STORED token (#308 R5b):
// `{ organization_id }`. Re-proves the admin and files its projects; a token
// Azure rejects marks the connection "needs a new token" (409). The body carries
// no token. ADMIN floor; system work.
import { dojoDb } from '$lib/server/dojo-supabase';
import { organizationRequest, syncAzureOrganization } from '$lib/server/azure-import';
import { azureRoute } from '$lib/server/azure-route';
import { azureAdapter } from '$lib/server/forge-azure';

export const POST = azureRoute(async ({ caller, request }) => {
	const { orgId } = await organizationRequest(request);
	return Response.json(
		await syncAzureOrganization(dojoDb(), caller.userId, caller.tenantId, orgId, (org) => azureAdapter(org))
	);
});
