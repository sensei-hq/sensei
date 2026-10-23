// GET /v1/t/{origin}/{org}/integrations/azure_devops — this dōjō's Azure DevOps
// organizations (#308 R5b): each one's stable id, name, when it was proven and
// its stored token's state (ok | needs_new_token | none) — never a token or a
// secret id — and the read-only scopes a token needs. ADMIN floor; the read is
// system work (tenant_connections has no session grant).
import { dojoDb } from '$lib/server/dojo-supabase';
import { listAzureConnections } from '$lib/server/azure-import';
import { azureRoute } from '$lib/server/azure-route';
import { AZURE_REQUIRED_SCOPES } from '$lib/server/forge-azure';

export const GET = azureRoute(async ({ caller }) =>
	Response.json({ connections: await listAzureConnections(dojoDb(), caller.tenantId), scopes: AZURE_REQUIRED_SCOPES })
);
