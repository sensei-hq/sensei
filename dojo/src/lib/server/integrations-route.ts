// The shape every Admin → Integrations route shares (#308 R5a): the admin floor
// of the screen's dōjō, the provider's adapter, and the person's GitHub token —
// taken ONLY from the sealed cookie a "Connect GitHub" re-authorization set
// (forge-token-cookie.ts), bound to this login. No cookie, a cookie that does not
// open, or a token GitHub answers 401 to: 428 "connect GitHub" (and the dead
// cookie is cleared). There is no other source of the token. The cookie exists
// only for a token GitHub already confirmed is this login's linked account (the
// check runs when it is sealed — captureForgeToken), so it is not re-checked here.
import type { RequestEvent } from '@sveltejs/kit';
import { AdminError, type DojoClient } from './admin-data';
import { ACCESS, apiError, type Caller } from './dojo-auth';
import { serverSecret } from './dojo-supabase';
import { forgeAdapter, type ForgeAdapter } from './forge-adapter';
import { ForgeReadError } from './forge-github';
import { clearForgeTokenCookie, FORGE_TOKEN_COOKIE, openForgeToken } from './forge-token-cookie';
import { providerParam } from './integrations-data';
import { tenantRoute } from './tenant-route';

export const NOT_CONNECTED = 'Connect GitHub to read your organizations';

type IntegrationsEvent = RequestEvent<Partial<Record<string, string>> & { origin: string; org: string }>;

export interface IntegrationsWork {
	caller: Caller;
	adapter: ForgeAdapter;
	/** The connected GitHub token — for this request's reads only. */
	token: string;
	request: Request;
}

export function integrationsRoute(work: (ctx: IntegrationsWork) => Promise<Response>) {
	return tenantRoute(ACCESS.admin, async (_db: DojoClient, caller, event: IntegrationsEvent) => {
		const adapter = forgeAdapter(providerParam(event.params.provider));
		const token = await openForgeToken(
			event.cookies.get(FORGE_TOKEN_COOKIE),
			caller.authUserId ?? '',
			serverSecret()
		);
		if (!token) throw new AdminError(428, NOT_CONNECTED);
		try {
			return await work({ caller, adapter, token, request: event.request });
		} catch (e) {
			if (e instanceof ForgeReadError && e.forgeStatus === 401) {
				const res = apiError(428, NOT_CONNECTED);
				res.headers.append('set-cookie', clearForgeTokenCookie(event.url.protocol === 'https:'));
				return res;
			}
			throw e;
		}
	});
}
