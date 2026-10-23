// POST /v1/integrations/azure_devops/import — import one organization the
// pending grant can see (#308 R5b, D36/D42): `{ organization }` (its name).
// Where it goes was sealed at start: the organization dōjō whose admin started
// it (re-checked), or the person's own new dōjō. The grant is validated before
// anything is written (azure-import.ts); its refresh token is stored in Vault
// for the connection. No token is answered. Rate-limited.
import { dojoDb, serverSecret } from '$lib/server/dojo-supabase';
import { AdminError } from '$lib/server/admin-data';
import { AZURE_GRANT_COOKIE, openGrant, useGrant } from '$lib/server/azure-connect';
import { importAzureOrganization, importRequest } from '$lib/server/azure-import';
import { azureConfig, azurePersonRoute, withCookies } from '$lib/server/azure-route';
import { azureAdapter } from '$lib/server/forge-azure';
import { guardAttempt } from '$lib/server/rate-limit';

export const POST = azurePersonRoute(async ({ person, event }) => {
	const grant = await openGrant(event.cookies.get(AZURE_GRANT_COOKIE), person.authUserId, serverSecret());
	if (!grant) throw new AdminError(428, 'Connect Azure DevOps first — the connection has expired or was never made');
	const { organization } = await importRequest(event.request);
	const cfg = azureConfig();
	const db = dojoDb();
	await guardAttempt(db, person.userId, 'azure_connect');
	const used = await useGrant({ cfg, grant, authUserId: person.authUserId, secret: serverSecret(), secure: event.url.protocol === 'https:' });
	const result = await importAzureOrganization(
		db,
		person.userId,
		grant.target,
		{ accessToken: used.accessToken, refreshToken: used.grant.refreshToken },
		azureAdapter(organization)
	);
	return withCookies(Response.json(result), [used.cookie]);
});
