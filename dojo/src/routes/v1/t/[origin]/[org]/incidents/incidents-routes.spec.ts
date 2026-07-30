// Route-level tests for the Worker lead incidents endpoints (JWT plane):
//   GET    /v1/t/{origin}/{org}/incidents        → { incidents, open_count }
//   POST   /v1/t/{origin}/{org}/incidents        → { id, severity }
//   PATCH  /v1/t/{origin}/{org}/incidents/{id}   → { id }
//   DELETE /v1/t/{origin}/{org}/incidents/{id}   → { deleted: true }
//
// On the JWT/console plane (`resolveTenantAccess`) at the LEAD floor. Covers the
// auth floor, each envelope, that every write audits + enforces the floor, the
// 404 on a missing row, error mapping, and auth-Response propagation. The
// `incidents-data` store is mocked (its own logic is in `incidents-data.spec.ts`).
// No live Worker/DB.
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { AdminError } from '$lib/server/admin-data';

const caller = { tenantId: 't1', userId: 'lead-uuid', role: 'lead', access: 2, membershipId: 'm1' };

class IncidentsError extends Error {
	constructor(
		readonly status: number,
		message: string
	) {
		super(message);
	}
}

const mocks = vi.hoisted(() => ({
	resolveTenantAccess: vi.fn(),
	listIncidents: vi.fn(),
	getIncidentDetail: vi.fn(),
	createIncident: vi.fn(),
	parseNewIncident: vi.fn(),
	updateIncident: vi.fn(),
	parsePatchIncident: vi.fn(),
	deleteIncident: vi.fn(),
	resolveEngagementClientNames: vi.fn(),
	recordAudit: vi.fn()
}));

vi.mock('$lib/server/dojo-supabase', () => ({
	dojoDb: () => ({}),
	ACCESS: { member: 0, contributor: 1, lead: 2, maintainer: 3, admin: 4 }
}));
vi.mock('$lib/server/dojo-auth', () => ({
	resolveTenantAccess: mocks.resolveTenantAccess,
	apiError: (status: number, message: string) =>
		new Response(JSON.stringify({ error: message }), { status, headers: { 'content-type': 'application/json' } }),
	ACCESS: { member: 0, contributor: 1, lead: 2, maintainer: 3, admin: 4 }
}));
vi.mock('$lib/server/incidents-data', () => ({
	listIncidents: mocks.listIncidents,
	getIncidentDetail: mocks.getIncidentDetail,
	createIncident: mocks.createIncident,
	parseNewIncident: mocks.parseNewIncident,
	updateIncident: mocks.updateIncident,
	parsePatchIncident: mocks.parsePatchIncident,
	deleteIncident: mocks.deleteIncident,
	IncidentsError
}));
// The engagement→client-name resolver is unit-tested in engagement-client-names.spec;
// mocked here so the GET test isolates the enrichment shaping.
vi.mock('$lib/server/engagement-client-names', () => ({
	resolveEngagementClientNames: mocks.resolveEngagementClientNames
}));
vi.mock('$lib/server/audit', () => ({ recordAudit: mocks.recordAudit }));

const list = await import('./+server');
const detail = await import('./[id]/+server');

function ev(body?: unknown, params: Record<string, string> = {}) {
	return {
		params: { origin: 'github', org: 'acme', ...params },
		request: new Request('http://x/', {
			method: body === undefined ? 'GET' : 'POST',
			headers: { authorization: 'Bearer jwt', 'content-type': 'application/json' },
			body: body === undefined ? undefined : JSON.stringify(body)
		}),
		locals: {},
		url: new URL('http://x/')
	} as never;
}

beforeEach(() => {
	mocks.resolveTenantAccess.mockClear().mockResolvedValue(caller);
	mocks.listIncidents.mockClear().mockResolvedValue({ incidents: [], open_count: 0 });
	mocks.getIncidentDetail.mockClear().mockResolvedValue({ id: 'i1', client_name: null, owner_name: null, artifact: null });
	mocks.createIncident.mockClear().mockResolvedValue({ id: 'i1', severity: 'high' });
	mocks.parseNewIncident.mockClear().mockReturnValue({ title: 'leak', severity: 'high' });
	mocks.updateIncident.mockClear().mockResolvedValue({ id: 'i1' });
	mocks.parsePatchIncident.mockClear().mockReturnValue({ resolve: true, status: 'resolved' });
	mocks.deleteIncident.mockClear().mockResolvedValue(true);
	mocks.resolveEngagementClientNames.mockClear().mockResolvedValue(new Map());
	mocks.recordAudit.mockClear().mockResolvedValue(undefined);
});

describe('GET /incidents', () => {
	it('returns { incidents, open_count } at the lead floor', async () => {
		mocks.listIncidents.mockResolvedValueOnce({ incidents: [{ id: 'i1' }], open_count: 1 });
		const res = await list.GET(ev());
		expect(await res.json()).toEqual({ incidents: [{ id: 'i1', client_name: null }], open_count: 1 });
		expect(mocks.resolveTenantAccess.mock.calls[0][4]).toBe(2);
	});
	it('maps IncidentsError(500) to 500', async () => {
		mocks.listIncidents.mockRejectedValueOnce(new IncidentsError(500, 'boom'));
		expect((await list.GET(ev())).status).toBe(500);
	});
	it('propagates a 403 from auth without querying', async () => {
		mocks.resolveTenantAccess.mockRejectedValueOnce(new Response('{}', { status: 403 }));
		expect((await list.GET(ev())).status).toBe(403);
		expect(mocks.listIncidents).not.toHaveBeenCalled();
	});
	it('enriches each incident with its resolved client name (engagement → client_name)', async () => {
		mocks.listIncidents.mockResolvedValueOnce({
			incidents: [
				{ id: 'i1', engagement_id: 'e1' },
				{ id: 'i2', engagement_id: null }
			],
			open_count: 1
		});
		mocks.resolveEngagementClientNames.mockResolvedValueOnce(new Map([['e1', 'Globex']]));
		const body = await (await list.GET(ev())).json();
		expect(body.incidents[0]).toMatchObject({ id: 'i1', client_name: 'Globex' });
		expect(body.incidents[1]).toMatchObject({ id: 'i2', client_name: null }); // unbound → null
	});
	it('surfaces a client-name resolve error as 500 (fail-closed)', async () => {
		mocks.listIncidents.mockResolvedValueOnce({ incidents: [{ id: 'i1', engagement_id: 'e1' }], open_count: 1 });
		mocks.resolveEngagementClientNames.mockRejectedValueOnce(new AdminError(500, 'boom'));
		expect((await list.GET(ev())).status).toBe(500);
	});
});

describe('GET /incidents/{id} (detail)', () => {
	it('returns the incident detail at the lead floor', async () => {
		mocks.getIncidentDetail.mockResolvedValueOnce({ id: 'i1', client_name: 'Globex', owner_name: 'Ada', artifact: null });
		const res = await detail.GET(ev(undefined, { id: 'i1' }));
		expect(await res.json()).toMatchObject({ id: 'i1', client_name: 'Globex', owner_name: 'Ada' });
		expect(mocks.resolveTenantAccess.mock.calls[0][4]).toBe(2); // ACCESS.lead
	});
	it('maps a 404 from the store', async () => {
		mocks.getIncidentDetail.mockRejectedValueOnce(new IncidentsError(404, 'no such incident'));
		expect((await detail.GET(ev(undefined, { id: 'ghost' }))).status).toBe(404);
	});
	it('maps an AdminError from name resolution to its status (fail-closed)', async () => {
		mocks.getIncidentDetail.mockRejectedValueOnce(new AdminError(500, 'resolve boom'));
		expect((await detail.GET(ev(undefined, { id: 'i1' }))).status).toBe(500);
	});
	it('propagates a 403 from auth', async () => {
		mocks.resolveTenantAccess.mockRejectedValueOnce(new Response('{}', { status: 403 }));
		expect((await detail.GET(ev(undefined, { id: 'i1' }))).status).toBe(403);
	});
});

describe('POST /incidents', () => {
	it('opens, audits, returns { id, severity } at the lead floor', async () => {
		const res = await list.POST(ev({ title: 'leak', severity: 'high' }));
		expect(await res.json()).toEqual({ id: 'i1', severity: 'high' });
		expect(mocks.resolveTenantAccess.mock.calls[0][4]).toBe(2);
		expect(mocks.recordAudit.mock.calls[0][3]).toMatchObject({ action: 'incident_opened', target: 'i1' });
	});
	it('maps a 400 from a bad body (no title) without auditing', async () => {
		mocks.parseNewIncident.mockImplementationOnce(() => {
			throw new IncidentsError(400, 'title is required');
		});
		expect((await list.POST(ev({}))).status).toBe(400);
		expect(mocks.recordAudit).not.toHaveBeenCalled();
	});
	it('propagates a 403 without writing', async () => {
		mocks.resolveTenantAccess.mockRejectedValueOnce(new Response('{}', { status: 403 }));
		expect((await list.POST(ev({ title: 'x' }))).status).toBe(403);
		expect(mocks.createIncident).not.toHaveBeenCalled();
	});
});

describe('PATCH /incidents/{id}', () => {
	it('resolves, audits (incident_resolved), returns { id }', async () => {
		const res = await detail.PATCH(ev({ resolved: true }, { id: 'i1' }));
		expect(await res.json()).toEqual({ id: 'i1' });
		expect(mocks.updateIncident.mock.calls[0][2]).toBe('i1');
		expect(mocks.recordAudit.mock.calls[0][3]).toMatchObject({ action: 'incident_resolved' });
	});
	it('maps a 404 from the store', async () => {
		mocks.updateIncident.mockRejectedValueOnce(new IncidentsError(404, 'no such incident'));
		expect((await detail.PATCH(ev({ status: 'open' }, { id: 'i9' }))).status).toBe(404);
	});
});

describe('DELETE /incidents/{id}', () => {
	it('deletes, audits, returns { deleted: true }', async () => {
		const res = await detail.DELETE(ev({}, { id: 'i1' }));
		expect(await res.json()).toEqual({ deleted: true });
		expect(mocks.recordAudit.mock.calls[0][3]).toMatchObject({ action: 'incident_deleted' });
	});
	it('404s (skips audit) when nothing matched', async () => {
		mocks.deleteIncident.mockResolvedValueOnce(false);
		expect((await detail.DELETE(ev({}, { id: 'i9' }))).status).toBe(404);
		expect(mocks.recordAudit).not.toHaveBeenCalled();
	});
});
