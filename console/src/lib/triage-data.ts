// Maintainer-console triage API client + types (R9).
//
// Binds to the real dojo-mind triage endpoints (crates/dojo-mind/src/api.rs):
//   GET    {DOJO}/v1/t/{tenant}/triage                     → { queue: TriageRow[] }
//   POST   {DOJO}/v1/t/{tenant}/triage/promote             → { promoted: PromoteResult[] }
//   POST   {DOJO}/v1/t/{tenant}/triage/{signature}/decide  → DecideResult
//
// Types are derived from the Rust structs (TriageRow, DecideBody, DecideOutcome,
// PromoteOutcome) in crates/dojo-mind/src/collective/promote.rs — not invented.
// The base URL is PUBLIC_DOJO_API_URL (console/.env.example, default :7755).
//
// Kept in a plain module (no top-level `fetch`, base URL passed in) so the types
// import cleanly in SSR/prerender and unit tests without a live backend; the
// screens render off an injected/empty fixture until Jerry wires a real session.

import { env } from '$env/dynamic/public';

// The dojo service base URL is deploy-specific (console/.env.example, default
// :7755). Read it from the dynamic public env so a build doesn't require the var
// to be present, and it can differ per deployment without a rebuild. Falls back
// to the local-dev default when unset.
const PUBLIC_DOJO_API_URL = env.PUBLIC_DOJO_API_URL ?? 'http://127.0.0.1:7755';

// ── wire types (mirror the Rust structs) ─────────────────────────────────────

/** One open triage row — `dojo_protocol` `TriageRow` (promote.rs:195). */
export interface TriageRow {
	signature: string;
	artifact_id: string;
	/** artifact kind — `guard | pattern | principle | prompt | skill | agent | persona | …` */
	kind: string;
	title: string;
	/** owner scope descriptor (opaque JSON; a `{ label?, kind? }`-ish object). */
	owner_scope: unknown;
	/** cluster confidence 0..1 (null before scoring). */
	confidence: number | null;
	contributor_count: number;
	/** similarity 0..1 to the nearest existing artifact (null when none). */
	similarity: number | null;
	nearest_artifact_id: string | null;
	/** `queued | in_review` (open states listed by the API). */
	state: string;
	/** ISO-8601 `YYYY-MM-DDTHH:MM:SSZ`. */
	created_at: string;
}

/** `GET …/triage` envelope. */
export interface TriageQueueResponse {
	queue: TriageRow[];
}

/** A maintainer verdict — `DecideBody` (api.rs:417). */
export type DecideStatus = 'approve' | 'revise' | 'decline';

export interface DecideBody {
	status: DecideStatus;
	/** required for `approve` (API 400s without it). */
	distribution_scope?: unknown;
	/** required (non-empty) for `decline` (API 400s without it). */
	reason?: string;
}

/**
 * `POST …/decide` result — the flattened `DecideOutcome` (api.rs:475). `status`
 * is the past-tense verb; `artifact_id`/`seq` are present only on `approved`.
 */
export interface DecideResult {
	status: 'approved' | 'declined' | 'revised';
	artifact_id?: string;
	seq?: number;
}

/** One entry of `POST …/promote` — `PromoteOutcome` (promote.rs:139) tagged by `outcome`. */
export interface PromoteResult {
	signature: string;
	result: {
		outcome: 'published' | 'merged' | 'queued' | 'no_op';
		artifact_id?: string;
		seq?: number;
		into?: string;
		reason?: string;
	};
}

/** `POST …/promote` envelope. */
export interface PromoteSweepResponse {
	promoted: PromoteResult[];
}

// ── client ───────────────────────────────────────────────────────────────────

/** Thrown for a non-2xx response; carries the status + the API `error` message. */
export class TriageApiError extends Error {
	constructor(
		public status: number,
		message: string
	) {
		super(message);
		this.name = 'TriageApiError';
	}
}

/** The base URL of the dojo service, without a trailing slash. */
export const dojoApiUrl = PUBLIC_DOJO_API_URL.replace(/\/$/, '');

/**
 * The `Authorization` header for a dojo call. Dojo routes are dual-auth: a
 * Supabase JWT (bearer) OR an API key. The console runs on the JWT plane — the
 * caller passes the session `access_token`. Returns `{}` when there's no token
 * (the request will 401; the screen surfaces that rather than crashing).
 */
function authHeaders(accessToken?: string | null): Record<string, string> {
	return accessToken ? { Authorization: `Bearer ${accessToken}` } : {};
}

async function parse<T>(res: Response): Promise<T> {
	let body: unknown = null;
	try {
		body = await res.json();
	} catch {
		// non-JSON body (e.g. a proxy 502) — fall through to the status message.
	}
	if (!res.ok) {
		const apiError =
			body && typeof body === 'object' && 'error' in body
				? String((body as { error: unknown }).error)
				: '';
		const msg = apiError || `request failed (${res.status})`;
		throw new TriageApiError(res.status, msg);
	}
	return body as T;
}

/**
 * The url-encoded tenant discovery path segment for `/v1/t/{tenant}/…`. The
 * tenant key is `<origin>/<org>[/<dojo>]`; each path segment is encoded so a
 * slash in the key survives as a real path separator.
 */
function encodeTenant(tenantKey: string): string {
	return tenantKey
		.split('/')
		.map((s) => encodeURIComponent(s))
		.join('/');
}

/** `GET …/triage` — the tenant's open triage rows. */
export async function listTriage(
	tenantKey: string,
	opts: { accessToken?: string | null; fetch?: typeof fetch } = {}
): Promise<TriageRow[]> {
	const f = opts.fetch ?? fetch;
	const res = await f(`${dojoApiUrl}/v1/t/${encodeTenant(tenantKey)}/triage`, {
		headers: authHeaders(opts.accessToken)
	});
	const data = await parse<TriageQueueResponse>(res);
	return data.queue ?? [];
}

/** `POST …/triage/promote` — run the tenant promotion sweep (idempotent). */
export async function promoteSweep(
	tenantKey: string,
	opts: { accessToken?: string | null; fetch?: typeof fetch } = {}
): Promise<PromoteResult[]> {
	const f = opts.fetch ?? fetch;
	const res = await f(`${dojoApiUrl}/v1/t/${encodeTenant(tenantKey)}/triage/promote`, {
		method: 'POST',
		headers: authHeaders(opts.accessToken)
	});
	const data = await parse<PromoteSweepResponse>(res);
	return data.promoted ?? [];
}

/**
 * `POST …/triage/{signature}/decide` — record a maintainer decision. `approve`
 * requires `distribution_scope`; `decline` requires a non-empty `reason` (both
 * validated server-side with a 400). The signature is url-encoded.
 */
export async function decideTriage(
	tenantKey: string,
	signature: string,
	body: DecideBody,
	opts: { accessToken?: string | null; fetch?: typeof fetch } = {}
): Promise<DecideResult> {
	const f = opts.fetch ?? fetch;
	const res = await f(
		`${dojoApiUrl}/v1/t/${encodeTenant(tenantKey)}/triage/${encodeURIComponent(signature)}/decide`,
		{
			method: 'POST',
			headers: { 'content-type': 'application/json', ...authHeaders(opts.accessToken) },
			body: JSON.stringify(body)
		}
	);
	return parse<DecideResult>(res);
}
