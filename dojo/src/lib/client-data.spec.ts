import { describe, expect, it, vi } from 'vitest';
import {
	ClientApiError,
	bindEngagementProject,
	createEngagement,
	createIncident,
	deleteEngagement,
	deleteIncident,
	dojoApiUrl,
	exportCompliance,
	getIncident,
	listAuditArtifacts,
	listEngagements,
	listIncidents,
	updateEngagement,
	updateIncident
} from '$lib/client-data';

// A fake fetch that records the call and returns a canned response. Mirrors the
// admin-data.spec / triage-data.spec harness (same shape) since all three clients
// share dojo-api. `text` optionally backs a non-JSON (CSV) body.
function fakeFetch(status: number, body: unknown, text?: string) {
	const calls: { url: string; init?: RequestInit }[] = [];
	const fn = vi.fn(async (url: string | URL, init?: RequestInit) => {
		calls.push({ url: String(url), init });
		return {
			ok: status >= 200 && status < 300,
			status,
			json: async () => body,
			text: async () => text ?? ''
		} as Response;
	});
	return { fn: fn as unknown as typeof fetch, calls };
}

function headersOf(init?: RequestInit): Record<string, string> {
	return (init?.headers ?? {}) as Record<string, string>;
}

describe('client-data reuses the shared dojo base url', () => {
	it('defaults to same-origin (empty base)', () => {
		expect(dojoApiUrl).toBe('');
	});
});

describe('listEngagements', () => {
	it('GETs the tenant engagements path and unwraps engagements', async () => {
		const engagements = [{ id: 'e1' }, { id: 'e2' }];
		const { fn, calls } = fakeFetch(200, { engagements });
		const out = await listEngagements('github/globex', { fetch: fn });
		expect(out).toEqual(engagements);
		expect(calls[0].url).toBe(`${dojoApiUrl}/v1/t/github/globex/engagements`);
		expect(calls[0].init?.method ?? 'GET').toBe('GET');
	});

	it('encodes each tenant segment but keeps the slash as a separator', async () => {
		const { fn, calls } = fakeFetch(200, { engagements: [] });
		await listEngagements('other/acme corp', { fetch: fn });
		expect(calls[0].url).toBe(`${dojoApiUrl}/v1/t/other/acme%20corp/engagements`);
	});

	it('sends a bearer header when an access token is given', async () => {
		const { fn, calls } = fakeFetch(200, { engagements: [] });
		await listEngagements('t/x', { fetch: fn, accessToken: 'jwt-123' });
		expect(headersOf(calls[0].init).Authorization).toBe('Bearer jwt-123');
	});

	it('omits the Authorization header when the token is null or absent', async () => {
		const nullTok = fakeFetch(200, { engagements: [] });
		await listEngagements('t/x', { fetch: nullTok.fn, accessToken: null });
		expect(headersOf(nullTok.calls[0].init).Authorization).toBeUndefined();

		const noTok = fakeFetch(200, { engagements: [] });
		await listEngagements('t/x', { fetch: noTok.fn });
		expect(headersOf(noTok.calls[0].init).Authorization).toBeUndefined();
	});

	it('returns an empty array when the envelope has no engagements', async () => {
		const { fn } = fakeFetch(200, {});
		expect(await listEngagements('t/x', { fetch: fn })).toEqual([]);
	});

	it('throws a ClientApiError carrying the API error message on a 403', async () => {
		const { fn } = fakeFetch(403, { error: 'lead role required' });
		await expect(listEngagements('t/x', { fetch: fn })).rejects.toMatchObject({
			name: 'DojoApiError',
			status: 403,
			message: 'lead role required'
		});
		const again = fakeFetch(403, { error: 'lead role required' });
		await expect(listEngagements('t/x', { fetch: again.fn })).rejects.toBeInstanceOf(ClientApiError);
	});
});

describe('createEngagement', () => {
	it('POSTs the engagement body with the bearer + json content-type', async () => {
		const { fn, calls } = fakeFetch(200, { id: 'e9' });
		const body = { client_name: 'Globex', description: 'auth work' };
		const out = await createEngagement('github/globex', body, { fetch: fn, accessToken: 'jwt' });
		expect(out).toEqual({ id: 'e9' });
		expect(calls[0].url).toBe(`${dojoApiUrl}/v1/t/github/globex/engagements`);
		expect(calls[0].init?.method).toBe('POST');
		expect(headersOf(calls[0].init)['content-type']).toBe('application/json');
		expect(headersOf(calls[0].init).Authorization).toBe('Bearer jwt');
		expect(JSON.parse(String(calls[0].init?.body))).toEqual(body);
	});

	it('surfaces the API 400 when client_name is missing', async () => {
		const { fn } = fakeFetch(400, { error: 'client_name is required' });
		await expect(
			createEngagement('t/x', { client_name: '' }, { fetch: fn })
		).rejects.toBeInstanceOf(ClientApiError);
	});
});

describe('updateEngagement', () => {
	it('PATCHes the engagement (id encoded) — close via status = ended', async () => {
		const { fn, calls } = fakeFetch(200, { id: 'e/1' });
		const out = await updateEngagement('github/globex', 'e/1', { status: 'ended' }, { fetch: fn });
		expect(out).toEqual({ id: 'e/1' });
		expect(calls[0].url).toBe(`${dojoApiUrl}/v1/t/github/globex/engagements/e%2F1`);
		expect(calls[0].init?.method).toBe('PATCH');
		expect(JSON.parse(String(calls[0].init?.body))).toEqual({ status: 'ended' });
	});
});

describe('bindEngagementProject', () => {
	it('POSTs the bind path with the project binding body', async () => {
		const { fn, calls } = fakeFetch(200, { id: 'e1', bound: true });
		const body = { project_id: 'p-42', name: 'ledger-core' };
		const out = await bindEngagementProject('t/x', 'e1', body, { fetch: fn });
		expect(out).toEqual({ id: 'e1', bound: true });
		expect(calls[0].url).toBe(`${dojoApiUrl}/v1/t/t/x/engagements/e1/bind`);
		expect(calls[0].init?.method).toBe('POST');
		expect(JSON.parse(String(calls[0].init?.body))).toEqual(body);
	});
});

describe('deleteEngagement', () => {
	it('DELETEs an engagement by id with no body', async () => {
		const { fn, calls } = fakeFetch(200, { deleted: true });
		expect(await deleteEngagement('t/x', 'e1', { fetch: fn })).toEqual({ deleted: true });
		expect(calls[0].init?.method).toBe('DELETE');
		expect(calls[0].init?.body).toBeUndefined();
	});
});

describe('listIncidents', () => {
	it('GETs the incidents path and unwraps incidents + open_count', async () => {
		const incidents = [{ id: 'i1' }, { id: 'i2' }];
		const { fn, calls } = fakeFetch(200, { incidents, open_count: 2 });
		const out = await listIncidents('github/globex', { fetch: fn });
		expect(out).toEqual({ incidents, open_count: 2 });
		expect(calls[0].url).toBe(`${dojoApiUrl}/v1/t/github/globex/incidents`);
	});

	it('defaults open_count to 0 and incidents to [] on a bare envelope', async () => {
		const { fn } = fakeFetch(200, {});
		expect(await listIncidents('t/x', { fetch: fn })).toEqual({ incidents: [], open_count: 0 });
	});
});

describe('createIncident', () => {
	it('POSTs the incident body with the bearer', async () => {
		const { fn, calls } = fakeFetch(200, { id: 'i9', severity: 'high' });
		const body = { title: 'near-leak', severity: 'high' as const };
		const out = await createIncident('t/x', body, { fetch: fn, accessToken: 'jwt' });
		expect(out).toEqual({ id: 'i9', severity: 'high' });
		expect(calls[0].init?.method).toBe('POST');
		expect(headersOf(calls[0].init).Authorization).toBe('Bearer jwt');
		expect(JSON.parse(String(calls[0].init?.body))).toEqual(body);
	});

	it('surfaces the API 400 for an invalid severity', async () => {
		const { fn } = fakeFetch(400, { error: 'severity must be low|medium|high|critical' });
		await expect(
			createIncident('t/x', { title: 'x', severity: 'nuclear' as never }, { fetch: fn })
		).rejects.toBeInstanceOf(ClientApiError);
	});
});

describe('updateIncident', () => {
	it('PATCHes the incident (id encoded) — resolve via resolved: true', async () => {
		const { fn, calls } = fakeFetch(200, { id: 'i1' });
		await updateIncident('t/x', 'i1', { resolved: true, resolution: 'contained' }, { fetch: fn });
		expect(calls[0].url).toBe(`${dojoApiUrl}/v1/t/t/x/incidents/i1`);
		expect(calls[0].init?.method).toBe('PATCH');
		expect(JSON.parse(String(calls[0].init?.body))).toEqual({
			resolved: true,
			resolution: 'contained'
		});
	});
});

describe('deleteIncident', () => {
	it('DELETEs an incident by id', async () => {
		const { fn, calls } = fakeFetch(200, { deleted: true });
		expect(await deleteIncident('t/x', 'i1', { fetch: fn })).toEqual({ deleted: true });
		expect(calls[0].init?.method).toBe('DELETE');
	});
});

describe('getIncident', () => {
	it('GETs the incident detail path (id encoded) and returns the detail', async () => {
		const detail = { id: 'i1', client_name: 'Globex', owner_name: 'Ada', artifact: null };
		const { fn, calls } = fakeFetch(200, detail);
		const out = await getIncident('t/x', 'i/1', { fetch: fn });
		expect(out).toEqual(detail);
		expect(calls[0].url).toBe(`${dojoApiUrl}/v1/t/t/x/incidents/i%2F1`);
		expect(calls[0].init?.method ?? 'GET').toBe('GET');
	});
	it('surfaces a 404 as a ClientApiError', async () => {
		const { fn } = fakeFetch(404, { error: 'no such incident' });
		await expect(getIncident('t/x', 'ghost', { fetch: fn })).rejects.toBeInstanceOf(ClientApiError);
	});
});

describe('listAuditArtifacts', () => {
	it('GETs the audit path with the engagement query and returns the bare array', async () => {
		const rows = [{ id: 'a1' }, { id: 'a2' }];
		const { fn, calls } = fakeFetch(200, rows);
		const out = await listAuditArtifacts('t/x', { fetch: fn, engagement: 'e1' });
		expect(out).toEqual(rows);
		expect(calls[0].url).toBe(`${dojoApiUrl}/v1/t/t/x/audit/artifacts?engagement=e1`);
	});

	it('sends the bearer header', async () => {
		const { fn, calls } = fakeFetch(200, []);
		await listAuditArtifacts('t/x', { fetch: fn, accessToken: 'jwt', engagement: 'e1' });
		expect(headersOf(calls[0].init).Authorization).toBe('Bearer jwt');
	});
});

describe('exportCompliance', () => {
	it('GETs csv by default and returns the raw csv text', async () => {
		const csv = 'artifact_id,client\na1,Globex\n';
		const { fn, calls } = fakeFetch(200, null, csv);
		const out = await exportCompliance('t/x', { fetch: fn, engagement: 'e1' });
		expect(out).toEqual({ format: 'csv', csv });
		expect(calls[0].url).toBe(
			`${dojoApiUrl}/v1/t/t/x/compliance/export?engagement=e1&format=csv`
		);
	});

	it('GETs json rows when format=json', async () => {
		const rows = [{ artifact_id: 'a1', client: 'Globex' }];
		const { fn, calls } = fakeFetch(200, { rows });
		const out = await exportCompliance('t/x', { fetch: fn, engagement: 'e1', format: 'json' });
		expect(out).toEqual({ format: 'json', rows });
		expect(calls[0].url).toContain('format=json');
	});

	it('surfaces a non-2xx error as a ClientApiError', async () => {
		const { fn } = fakeFetch(500, { error: 'internal error' });
		await expect(
			exportCompliance('t/x', { fetch: fn, engagement: 'e1' })
		).rejects.toBeInstanceOf(ClientApiError);
	});
});
