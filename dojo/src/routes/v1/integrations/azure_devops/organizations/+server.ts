// POST /v1/integrations/azure_devops/organizations — the organizations the
// pending Azure DevOps grant can see (#308 R5b, D42), the write scopes it carries
// (the screen warns), and where an import goes (`this` dōjō or their `own`).
// The grant is the sealed cookie the callback set, for this login only (428
// without it); using it refreshes and re-seals it. No token is answered.
// Rate-limited.
import { dojoDb, serverSecret } from '$lib/server/dojo-supabase';
import { AdminError } from '$lib/server/admin-data';
import { AZURE_GRANT_COOKIE, openGrant, useGrant } from '$lib/server/azure-connect';
import { azureConfig, azurePersonRoute, withCookies } from '$lib/server/azure-route';
import { listAzureOrganizations } from '$lib/server/forge-azure';
import { guardAttempt } from '$lib/server/rate-limit';

export const POST = azurePersonRoute(async ({ person, event }) => {
	const grant = await openGrant(event.cookies.get(AZURE_GRANT_COOKIE), person.authUserId, serverSecret());
	if (!grant) throw new AdminError(428, 'Connect Azure DevOps first — the connection has expired or was never made');
	const cfg = azureConfig();
	await guardAttempt(dojoDb(), person.userId, 'azure_connect');
	const used = await useGrant({ cfg, grant, authUserId: person.authUserId, secret: serverSecret(), secure: event.url.protocol === 'https:' });
	const read = await listAzureOrganizations(used.accessToken);
	return withCookies(
		Response.json({ organizations: read.organizations, complete: read.complete, writeScopes: grant.writeScopes, target: grant.target.as }),
		[used.cookie]
	);
});
