// Shared HTTP primitives for the dojo-mind API clients (triage-data, admin-data).
//
// The console talks to one service (crates/dojo-mind) over one auth model, so the
// base URL, the bearer-header shape, the tenant-path encoding, the error type and
// the response parser are owned HERE and reused by every client — not duplicated
// per client. `triage-data.ts` re-exports `TriageApiError` + `dojoApiUrl` so its
// existing importers are unaffected.
//
// Kept in a plain module (no top-level `fetch`, base URL read once from the
// dynamic public env) so the types import cleanly under SSR/prerender and in unit
// tests without a live backend.

import { env } from '$env/dynamic/public';

// The dojo service base URL is deploy-specific (console/.env.example, default
// :7755). Read it from the dynamic public env so a build doesn't require the var
// to be present and it can differ per deployment without a rebuild; fall back to
// the local-dev default when unset.
const PUBLIC_DOJO_API_URL = env.PUBLIC_DOJO_API_URL ?? 'http://127.0.0.1:7755';

/** The base URL of the dojo service, without a trailing slash. */
export const dojoApiUrl = PUBLIC_DOJO_API_URL.replace(/\/$/, '');

/** Thrown for a non-2xx response; carries the status + the API `error` message. */
export class DojoApiError extends Error {
	constructor(
		public status: number,
		message: string
	) {
		super(message);
		this.name = 'DojoApiError';
	}
}

/**
 * The `Authorization` header for a dojo call. Dojo routes are dual-auth: a
 * Supabase JWT (bearer) OR an API key. The console runs on the JWT plane — the
 * caller passes the session `access_token`. Returns `{}` when there's no token
 * (the request will 401; the screen surfaces that rather than crashing).
 */
export function authHeaders(accessToken?: string | null): Record<string, string> {
	return accessToken ? { Authorization: `Bearer ${accessToken}` } : {};
}

/**
 * Parse a JSON response, throwing a {@link DojoApiError} (status + API `error`
 * message) on any non-2xx. A non-JSON body (e.g. a proxy 502) falls back to a
 * status message rather than throwing on the JSON parse.
 */
export async function parseJson<T>(res: Response): Promise<T> {
	let body: unknown = null;
	try {
		body = await res.json();
	} catch {
		// non-JSON body — fall through to the status message.
	}
	if (!res.ok) {
		const apiError =
			body && typeof body === 'object' && 'error' in body
				? String((body as { error: unknown }).error)
				: '';
		const msg = apiError || `request failed (${res.status})`;
		throw new DojoApiError(res.status, msg);
	}
	return body as T;
}

/**
 * The url-encoded tenant discovery path segment for `/v1/t/{tenant}/…`. The
 * tenant key is `<origin>/<org>[/<dojo>]`; each path segment is encoded so a
 * slash in the key survives as a real path separator.
 */
export function encodeTenant(tenantKey: string): string {
	return tenantKey
		.split('/')
		.map((s) => encodeURIComponent(s))
		.join('/');
}

/** Options common to every client call: the session token + an injectable fetch. */
export interface DojoCallOpts {
	accessToken?: string | null;
	fetch?: typeof fetch;
}
