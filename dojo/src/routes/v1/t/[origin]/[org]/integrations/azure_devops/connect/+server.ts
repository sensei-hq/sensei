// POST /v1/t/{origin}/{org}/integrations/azure_devops/connect — start
// connecting Azure DevOps through Microsoft Entra (#308 R5b, D42) for THIS
// organization dōjō: answers `{ url }` (Entra's authorize URL) and sets the
// sealed state cookie bound to the admin's login. The imported organizations
// connect to this dōjō, each project a workspace (D36). ADMIN floor; rate-limited;
// 503 when the app registration is not configured.
import { dojoDb, serverSecret } from '$lib/server/dojo-supabase';
import { AdminError } from '$lib/server/admin-data';
import { startConnect } from '$lib/server/azure-connect';
import { azureConfig, azureRoute, withCookies } from '$lib/server/azure-route';
import { guardAttempt } from '$lib/server/rate-limit';

export const POST = azureRoute(async ({ caller, event }) => {
	if (event.params.origin !== 'organization') {
		throw new AdminError(400, 'an Azure DevOps organization is imported into an organization’s dōjō — from your personal dōjō, import it as its own');
	}
	if (!caller.authUserId) throw new AdminError(401, 'a signed-in session is required');
	const cfg = azureConfig();
	await guardAttempt(dojoDb(), caller.userId, 'azure_connect');
	const { url, cookie } = await startConnect({
		cfg,
		authUserId: caller.authUserId,
		secret: serverSecret(),
		origin: event.url.origin,
		target: { as: 'this', tenantId: caller.tenantId }
	});
	return withCookies(Response.json({ url }), [cookie]);
});
