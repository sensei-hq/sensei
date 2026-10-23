// POST /v1/you/integrations/azure_devops/connect — the personal dōjō's import
// (#308 R5b, D36): `{ organization_url, token }`. How an Azure DevOps
// organization's dōjō is first made: the token is validated first (it names the
// organization and proves its owner a Project Collection Administrator), then the
// organization's own dōjō is made — or joined, as a proven admin — and claimed,
// the token stored in Vault and each project filed as a workspace. A failing
// token is refused (422) and never stored. Any signed-in person; system work.
import { dojoDb } from '$lib/server/dojo-supabase';
import { apiError, resolveCaller } from '$lib/server/dojo-auth';
import { AdminError } from '$lib/server/admin-data';
import { connectRequest, importAzureOrganization } from '$lib/server/azure-import';
import { azureRefusal } from '$lib/server/azure-route';
import { azureAdapter } from '$lib/server/forge-azure';
import type { RequestHandler } from './$types';

export const POST: RequestHandler = async ({ request, locals }) => {
	try {
		const { userId } = await resolveCaller(request, locals);
		const { organization, token } = await connectRequest(request);
		return Response.json(await importAzureOrganization(dojoDb(), userId, { as: 'own' }, token, azureAdapter(organization)));
	} catch (raw) {
		const e = azureRefusal(raw);
		if (e instanceof Response) return e;
		if (e instanceof AdminError) return apiError(e.status, e.message);
		throw e;
	}
};
