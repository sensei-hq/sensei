import { describe, expect, it, vi } from 'vitest';
import {
	TriageApiError,
	decideTriage,
	dojoApiUrl,
	listTriage,
	promoteSweep,
	type DecideBody
} from '$lib/triage-data';

// A fake fetch that records the call and returns a canned JSON response.
function fakeFetch(status: number, body: unknown) {
	const calls: { url: string; init?: RequestInit }[] = [];
	const fn = vi.fn(async (url: string | URL, init?: RequestInit) => {
		calls.push({ url: String(url), init });
		return {
			ok: status >= 200 && status < 300,
			status,
			json: async () => body
		} as Response;
	});
	return { fn: fn as unknown as typeof fetch, calls };
}

describe('dojoApiUrl', () => {
	it('falls back to the local-dev default when the env is unset', () => {
		// The vitest stub supplies an empty env → the client uses the default base.
		expect(dojoApiUrl).toBe('http://127.0.0.1:7755');
	});
});

describe('listTriage', () => {
	it('GETs the tenant triage path and unwraps the queue', async () => {
		const rows = [{ signature: 's1' }, { signature: 's2' }];
		const { fn, calls } = fakeFetch(200, { queue: rows });
		const out = await listTriage('github/globex', { fetch: fn });
		expect(out).toEqual(rows);
		expect(calls[0].url).toBe(`${dojoApiUrl}/v1/t/github/globex/triage`);
	});

	it('encodes each tenant segment but keeps the slash as a separator', async () => {
		const { fn, calls } = fakeFetch(200, { queue: [] });
		await listTriage('other/acme corp', { fetch: fn });
		expect(calls[0].url).toBe(`${dojoApiUrl}/v1/t/other/acme%20corp/triage`);
	});

	it('sends a bearer header when an access token is given', async () => {
		const { fn, calls } = fakeFetch(200, { queue: [] });
		await listTriage('t/x', { fetch: fn, accessToken: 'jwt-123' });
		const headers = calls[0].init?.headers as Record<string, string>;
		expect(headers.Authorization).toBe('Bearer jwt-123');
	});

	it('omits the Authorization header when the token is null or absent', async () => {
		const nullToken = fakeFetch(200, { queue: [] });
		await listTriage('t/x', { fetch: nullToken.fn, accessToken: null });
		const nullHeaders = (nullToken.calls[0].init?.headers ?? {}) as Record<string, string>;
		expect(nullHeaders.Authorization).toBeUndefined();

		const noToken = fakeFetch(200, { queue: [] });
		await listTriage('t/x', { fetch: noToken.fn });
		const noHeaders = (noToken.calls[0].init?.headers ?? {}) as Record<string, string>;
		expect(noHeaders.Authorization).toBeUndefined();
	});

	it('returns an empty array when the envelope has no queue', async () => {
		const { fn } = fakeFetch(200, {});
		expect(await listTriage('t/x', { fetch: fn })).toEqual([]);
	});

	it('throws a TriageApiError carrying the API error message on non-2xx', async () => {
		const { fn } = fakeFetch(403, { error: 'maintainer role required' });
		await expect(listTriage('t/x', { fetch: fn })).rejects.toMatchObject({
			name: 'TriageApiError',
			status: 403,
			message: 'maintainer role required'
		});
	});
});

describe('promoteSweep', () => {
	it('POSTs the promote path and unwraps promoted', async () => {
		const promoted = [{ signature: 's1', result: { outcome: 'published' } }];
		const { fn, calls } = fakeFetch(200, { promoted });
		const out = await promoteSweep('github/globex', { fetch: fn });
		expect(out).toEqual(promoted);
		expect(calls[0].url).toBe(`${dojoApiUrl}/v1/t/github/globex/triage/promote`);
		expect(calls[0].init?.method).toBe('POST');
	});
});

describe('decideTriage', () => {
	it('POSTs the decide body to the signature path (signature encoded)', async () => {
		const { fn, calls } = fakeFetch(200, { status: 'approved', artifact_id: 'a1', seq: 7 });
		const body: DecideBody = { status: 'approve', distribution_scope: { label: 'Company' } };
		const out = await decideTriage('github/globex', 'sig/with slash', body, { fetch: fn });
		expect(out).toEqual({ status: 'approved', artifact_id: 'a1', seq: 7 });
		expect(calls[0].url).toBe(
			`${dojoApiUrl}/v1/t/github/globex/triage/sig%2Fwith%20slash/decide`
		);
		expect(JSON.parse(String(calls[0].init?.body))).toEqual(body);
	});

	it('surfaces the API 400 for an approve missing a distribution scope', async () => {
		const { fn } = fakeFetch(400, { error: 'approve requires distribution_scope' });
		await expect(
			decideTriage('t/x', 'sig', { status: 'approve' }, { fetch: fn })
		).rejects.toBeInstanceOf(TriageApiError);
	});
});
