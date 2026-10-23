// The shape every Azure DevOps Integrations route shares (#308 R5b): the admin
// floor of the screen's dōjō, and one answer for a failed Azure read. The PAT
// arrives only in a connect request's body; afterwards it lives in Vault and is
// read by the server alone. No answer and no error ever carries it.
import type { RequestEvent } from '@sveltejs/kit';
import { AdminError, type DojoClient } from './admin-data';
import { ACCESS, type Caller } from './dojo-auth';
import { ForgeReadError } from './forge-github';
import { tenantRoute } from './tenant-route';

/** What a person is told when Azure DevOps will not take the token. */
export const TOKEN_REFUSED =
	'Azure DevOps did not accept that token for this organization — check the organization URL, that the token has not expired, and that it has the read scopes listed';

/**
 * A failed Azure read as the caller's answer: a token Azure will not take (401,
 * 403, or the sign-in page it answers with — 203 or a redirect) is a 422 the
 * person can fix; a name Azure does not know is a 404; anything else stays the
 * 502 it is — an outage is not a bad token. Anything that is not an Azure read
 * error is returned unchanged.
 */
export function azureRefusal(e: unknown): unknown {
	if (!(e instanceof ForgeReadError)) return e;
	const s = e.forgeStatus;
	if (s === 401 || s === 403 || s === 203 || (s >= 300 && s < 400)) return new AdminError(422, TOKEN_REFUSED);
	if (s === 404) return new AdminError(404, 'Azure DevOps has no organization by that name, or this token cannot see it');
	return e;
}

type AzureEvent = RequestEvent<Partial<Record<string, string>> & { origin: string; org: string }>;

export interface AzureWork {
	caller: Caller;
	request: Request;
}

/** An Admin → Integrations → Azure DevOps route: the admin floor of the
 *  screen's dōjō, with Azure read failures answered by `azureRefusal`. */
export function azureRoute(work: (ctx: AzureWork) => Promise<Response>) {
	return tenantRoute(ACCESS.admin, async (_db: DojoClient, caller, event: AzureEvent) => {
		try {
			return await work({ caller, request: event.request });
		} catch (e) {
			throw azureRefusal(e);
		}
	});
}
