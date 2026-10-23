// The shape every Azure DevOps Integrations route shares (#308 R5b, D42): the
// app registration from the server env (503 without it), the admin floor of the
// screen's dōjō or the signed-in person, one answer for a failed Azure read, and
// cookies appended to the answer. Tokens arrive only from Microsoft (the connect
// callback) and live in sealed cookies or Vault; no request body carries one and
// no answer or error returns one.
import type { RequestEvent } from '@sveltejs/kit';
import { env as priv } from '$env/dynamic/private';
import { AdminError, type DojoClient } from './admin-data';
import { clearGrantCookie } from './azure-connect';
import { azureEntraConfig, AzureGrantRejected, type AzureEntraConfig } from './azure-entra';
import { ACCESS, apiError, resolveCaller, type Caller } from './dojo-auth';
import { ForgeReadError } from './forge-github';
import { tenantRoute } from './tenant-route';

/** The app registration, from the server env — or the 503 "not configured". */
export function azureConfig(): AzureEntraConfig {
	return azureEntraConfig(priv);
}

/** Whether the app registration is configured (the screen says so). */
export function azureConfigured(): boolean {
	try {
		azureConfig();
		return true;
	} catch {
		return false;
	}
}

/** What a person is told when Azure DevOps refuses a read with a fresh token. */
export const READ_REFUSED =
	'Azure DevOps refused the read for this organization — check that your account still belongs to it and has the read access the connect asked for';

/**
 * A failed Azure read as the caller's answer: refused (401, 403, or the sign-in
 * page Azure answers with — 203 or a redirect) is a 422 the person can act on; a
 * name Azure does not know is a 404; anything else stays the 502 it is — an
 * outage is not a refusal. Anything that is not an Azure read error is returned
 * unchanged.
 */
export function azureRefusal(e: unknown): unknown {
	if (!(e instanceof ForgeReadError)) return e;
	const s = e.forgeStatus;
	if (s === 401 || s === 403 || s === 203 || (s >= 300 && s < 400)) return new AdminError(422, READ_REFUSED);
	if (s === 404) return new AdminError(404, 'Azure DevOps has no organization by that name, or your account cannot see it');
	return e;
}

/** The answer, with these Set-Cookie values appended. */
export function withCookies(res: Response, cookies: string[]): Response {
	if (!cookies.length) return res;
	const headers = new Headers(res.headers);
	for (const c of cookies) headers.append('set-cookie', c);
	return new Response(res.body, { status: res.status, statusText: res.statusText, headers });
}

type AzureEvent = RequestEvent<Partial<Record<string, string>> & { origin: string; org: string }>;

export interface AzureWork {
	caller: Caller;
	event: AzureEvent;
}

/** An Admin → Integrations → Azure DevOps route: the admin floor of the
 *  screen's dōjō, with Azure read failures answered by `azureRefusal`. */
export function azureRoute(work: (ctx: AzureWork) => Promise<Response>) {
	return tenantRoute(ACCESS.admin, async (_db: DojoClient, caller, event: AzureEvent) => {
		try {
			return await work({ caller, event });
		} catch (e) {
			throw azureRefusal(e);
		}
	});
}

export interface AzurePersonWork {
	person: Awaited<ReturnType<typeof resolveCaller>>;
	event: RequestEvent;
}

/** A route for the signed-in person (the personal dōjō's connect, and the
 *  pending grant's organization list and import). A dead pending grant clears
 *  its cookie. */
export function azurePersonRoute(work: (ctx: AzurePersonWork) => Promise<Response>) {
	return async (event: RequestEvent): Promise<Response> => {
		try {
			const person = await resolveCaller(event.request, event.locals);
			return await work({ person, event });
		} catch (raw) {
			const e = azureRefusal(raw);
			if (e instanceof Response) return e;
			if (e instanceof AzureGrantRejected) {
				return withCookies(apiError(e.status, e.message), [clearGrantCookie(event.url.protocol === 'https:')]);
			}
			if (e instanceof AdminError) return apiError(e.status, e.message);
			throw e;
		}
	};
}
