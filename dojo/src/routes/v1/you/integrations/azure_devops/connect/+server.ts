// POST /v1/you/integrations/azure_devops/connect — the personal dōjō starts
// connecting Azure DevOps through Microsoft Entra (#308 R5b, D42): answers
// `{ url }` and sets the sealed state cookie. The organizations the person then
// imports become their own dōjōs — how an Azure organization's dōjō is first
// made (only for a proven Project Collection Administrator). Any signed-in
// person; rate-limited; 503 when the app registration is not configured.
import { dojoDb, serverSecret } from '$lib/server/dojo-supabase';
import { startConnect } from '$lib/server/azure-connect';
import { azureConfig, azurePersonRoute, withCookies } from '$lib/server/azure-route';
import { guardAttempt } from '$lib/server/rate-limit';

export const POST = azurePersonRoute(async ({ person, event }) => {
	const cfg = azureConfig();
	await guardAttempt(dojoDb(), person.userId, 'azure_connect');
	const { url, cookie } = await startConnect({
		cfg,
		authUserId: person.authUserId,
		secret: serverSecret(),
		origin: event.url.origin,
		target: { as: 'own' }
	});
	return withCookies(Response.json({ url }), [cookie]);
});
