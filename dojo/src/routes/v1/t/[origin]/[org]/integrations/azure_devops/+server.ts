// GET /v1/t/{origin}/{org}/integrations/azure_devops — this dōjō's Azure DevOps
// organizations (#308 R5b, D42): each one's stable id, name, when it was proven
// and its stored grant's state (ok | needs_new_token | none) — never a token or a
// secret id — the read scopes the connect asks for, and whether this server has
// an app registration configured (`configured`). ADMIN floor; system work.
import { dojoDb } from '$lib/server/dojo-supabase';
import { listAzureConnections } from '$lib/server/azure-import';
import { azureConfigured, azureRoute } from '$lib/server/azure-route';
import { AZURE_REQUIRED_SCOPES } from '$lib/server/forge-azure';

export const GET = azureRoute(async ({ caller }) =>
	Response.json({
		connections: await listAzureConnections(dojoDb(), caller.tenantId),
		scopes: AZURE_REQUIRED_SCOPES,
		configured: azureConfigured()
	})
);
