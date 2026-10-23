// A fetch stand-in for the forge reads, for tests: it answers by URL substring
// and records every call, so a spec can assert both what it was told and what it
// asked. Test-only — no network is ever reached through it.
//
// An unrouted URL answers 404, which is a real forge answer ("not visible to this
// token"), so a spec that forgets a route sees the not-found path rather than a
// fabricated success.
//
// A route may set response `headers` (Azure DevOps pages some lists with an
// `X-MS-ContinuationToken` header); with none, the answer is `application/json`,
// as a forge's API answers.

export interface FakeRoute {
	status?: number;
	body: unknown;
	headers?: Record<string, string>;
}

/** A fetch stub that answers by URL substring (the first route key the URL
 *  contains) and records the calls. */
export function fakeFetch(routes: Record<string, FakeRoute>) {
	const calls: { url: string; init?: RequestInit }[] = [];
	const fn = (async (url: string | URL | Request, init?: RequestInit) => {
		const u = String(url);
		calls.push({ url: u, init });
		const key = Object.keys(routes).find((k) => u.includes(k));
		const route = key ? routes[key] : undefined;
		const status = route?.status ?? (route ? 200 : 404);
		return {
			ok: status >= 200 && status < 300,
			status,
			headers: new Headers(route?.headers ?? { 'content-type': 'application/json; charset=utf-8' }),
			json: async () => route?.body ?? null
		} as Response;
	}) as unknown as typeof fetch;
	return { fn, calls };
}
