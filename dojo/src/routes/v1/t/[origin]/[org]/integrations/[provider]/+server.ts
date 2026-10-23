// POST /v1/t/{origin}/{org}/integrations/{provider} — Admin → Integrations
// (#308 R5a): the units (GitHub orgs) the person's connected GitHub can see, each
// with what it is today and what this dōjō's admin may do with it. ADMIN floor;
// the token is the "Connect GitHub" connection (428 without it); the reads are
// system work (every tenant's connections), named. Fails closed.
import { dojoDb } from '$lib/server/dojo-supabase';
import { listIntegrationUnits } from '$lib/server/integrations-data';
import { integrationsRoute } from '$lib/server/integrations-route';

export const POST = integrationsRoute(async ({ caller, adapter, token }) =>
	Response.json(await listIntegrationUnits(dojoDb(), caller.userId, caller.tenantId, token, adapter))
);
