// GET /v1/integrations/azure_devops/callback — Microsoft Entra's redirect back
// (#308 R5b, D42). The redirect URI registered on the app registration. A
// top-level navigation, so the person is identified by their dōjō session cookie
// and every outcome is a 303 back to the screen, naming why when it did not
// connect (?azure_error=…). The state must open for THIS login and match;
// only then is the code redeemed and the refresh token sealed as the pending
// grant. Rate-limited.
import type { RequestHandler } from './$types';
import { dojoDb, serverSecret } from '$lib/server/dojo-supabase';
import { resolveCaller } from '$lib/server/dojo-auth';
import { AZURE_STATE_COOKIE, finishConnect, type AzureConnectError } from '$lib/server/azure-connect';
import { azureConfig } from '$lib/server/azure-route';
import { guardAttempt } from '$lib/server/rate-limit';

const back = (error: AzureConnectError) =>
	new Response(null, { status: 303, headers: { location: `/you/dojos?azure_error=${error}` } });

export const GET: RequestHandler = async (event) => {
	let person;
	try {
		person = await resolveCaller(event.request, event.locals);
	} catch {
		return back('signin');
	}
	let cfg;
	try {
		cfg = azureConfig();
	} catch {
		return back('not_configured');
	}
	try {
		await guardAttempt(dojoDb(), person.userId, 'azure_connect');
	} catch {
		return back('too_many');
	}
	const out = await finishConnect({
		cfg,
		authUserId: person.authUserId,
		secret: serverSecret(),
		url: event.request.url,
		stateCookie: event.cookies.get(AZURE_STATE_COOKIE)
	});
	const headers = new Headers({ location: out.location });
	for (const c of out.cookies) headers.append('set-cookie', c);
	return new Response(null, { status: 303, headers });
};
