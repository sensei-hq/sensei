// DELETE /v1/t/{origin}/{org}/integrations/azure_devops/token — remove the stored
// token of one of this dōjō's Azure DevOps organizations (#308 R5b):
// `{ organization_id }`. Its Vault secret is deleted and the removal audited;
// member reads stop until an admin pastes a new one. ADMIN floor; system work.
import { dojoDb } from '$lib/server/dojo-supabase';
import { organizationRequest, removeAzureToken } from '$lib/server/azure-import';
import { azureRoute } from '$lib/server/azure-route';
import { noContent } from '$lib/server/tenant-route';

export const DELETE = azureRoute(async ({ caller, request }) => {
	const { orgId } = await organizationRequest(request);
	await removeAzureToken(dojoDb(), caller.userId, caller.tenantId, orgId);
	return noContent();
});
