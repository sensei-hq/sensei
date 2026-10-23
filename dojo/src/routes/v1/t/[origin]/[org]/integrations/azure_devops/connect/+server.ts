// POST /v1/t/{origin}/{org}/integrations/azure_devops/connect — Connect Azure
// DevOps (#308 R5b, D35/D36): `{ organization_url, token }`. The token is
// validated first (it names the organization and proves a Project Collection
// Administrator); only then is the organization connected to THIS dōjō and
// claimed, the token stored in Vault, and each project filed as a workspace. A
// failing token is refused (422) and never stored. ADMIN floor; system work.
import { dojoDb } from '$lib/server/dojo-supabase';
import { connectRequest, importAzureOrganization } from '$lib/server/azure-import';
import { azureRoute } from '$lib/server/azure-route';
import { azureAdapter } from '$lib/server/forge-azure';

export const POST = azureRoute(async ({ caller, request }) => {
	const { organization, token } = await connectRequest(request);
	return Response.json(
		await importAzureOrganization(dojoDb(), caller.userId, { as: 'this', tenantId: caller.tenantId }, token, azureAdapter(organization))
	);
});
