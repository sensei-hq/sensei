// Route-level tests for the Worker lead engagements detail/bind endpoints (JWT
// plane):
//   PATCH  /v1/t/{origin}/{org}/engagements/{id}        → { id }
//   DELETE /v1/t/{origin}/{org}/engagements/{id}        → { deleted: true }
//   POST   /v1/t/{origin}/{org}/engagements/{id}/bind   → { id, bound: true }
//
// On the JWT/console plane at the LEAD floor. Covers the auth floor, each
// envelope, that every write audits, the 404 on a missing row, error mapping, and
// auth-Response propagation. The `engagements-data` store is mocked (its own logic
// is in `engagements-data.spec.ts`). No live Worker/DB. (GET/POST list+create are
// inline in the parent `+server.ts` and unaffected.)
import { describe, it, expect, vi, beforeEach } from 'vitest';

const caller = { tenantId: 't1', userId: 'lead-uuid', role: 'lead', access: 2, membershipId: 'm1' };

class EngagementsError extends Error {
	constructor(
		readonly status: number,
		message: string
	) {
		super(message);
	}
}

const mocks = vi.hoisted(() => ({
	resolveTenantAccess: vi.fn(),
	updateEngagement: vi.fn(),
	parsePatchEngagement: vi.fn(),
	deleteEngagement: vi.fn(),
	bindEngagementProject: vi.fn(),
	parseBindProject: vi.fn(),
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
vi.mock('$lib/server/engagements-data', () => ({
	updateEngagement: mocks.updateEngagement,
	parsePatchEngagement: mocks.parsePatchEngagement,
	deleteEngagement: mocks.deleteEngagement,
	bindEngagementProject: mocks.bindEngagementProject,
	parseBindProject: mocks.parseBindProject,
	EngagementsError
}));
vi.mock('$lib/server/audit', () => ({ recordAudit: mocks.recordAudit }));

const detail = await import('./[id]/+server');
const bind = await import('./[id]/bind/+server');

function ev(body: unknown, params: Record<string, string> = {}) {
	return {
		params: { origin: 'github', org: 'acme', ...params },
		request: new Request('http://x/', {
			method: 'POST',
			headers: { authorization: 'Bearer jwt', 'content-type': 'application/json' },
			body: JSON.stringify(body)
		}),
		locals: {},
		url: new URL('http://x/')
	} as never;
}

beforeEach(() => {
	mocks.resolveTenantAccess.mockClear().mockResolvedValue(caller);
	mocks.updateEngagement.mockClear().mockResolvedValue({ id: 'e1' });
	mocks.parsePatchEngagement.mockClear().mockReturnValue({ status: 'ended' });
	mocks.deleteEngagement.mockClear().mockResolvedValue(true);
	mocks.bindEngagementProject.mockClear().mockResolvedValue({ id: 'e1', bound: true });
	mocks.parseBindProject.mockClear().mockReturnValue({ project_id: 'p1', name: 'One' });
	mocks.recordAudit.mockClear().mockResolvedValue(undefined);
});

describe('PATCH /engagements/{id}', () => {
	it('closes (status ended), audits engagement_closed, returns { id } at the lead floor', async () => {
		const res = await detail.PATCH(ev({ status: 'ended' }, { id: 'e1' }));
		expect(await res.json()).toEqual({ id: 'e1' });
		expect(mocks.resolveTenantAccess.mock.calls[0][4]).toBe(2);
		expect(mocks.recordAudit.mock.calls[0][3]).toMatchObject({ action: 'engagement_closed', engagementId: 'e1' });
	});
	it('audits engagement_updated for a non-close patch', async () => {
		mocks.parsePatchEngagement.mockReturnValueOnce({ description: 'x' });
		await detail.PATCH(ev({ description: 'x' }, { id: 'e1' }));
		expect(mocks.recordAudit.mock.calls[0][3]).toMatchObject({ action: 'engagement_updated' });
	});
	it('maps a 404 from the store', async () => {
		mocks.updateEngagement.mockRejectedValueOnce(new EngagementsError(404, 'no such engagement'));
		expect((await detail.PATCH(ev({ status: 'ended' }, { id: 'e9' }))).status).toBe(404);
	});
	it('propagates a 403 without writing', async () => {
		mocks.resolveTenantAccess.mockRejectedValueOnce(new Response('{}', { status: 403 }));
		expect((await detail.PATCH(ev({ status: 'ended' }, { id: 'e1' }))).status).toBe(403);
		expect(mocks.updateEngagement).not.toHaveBeenCalled();
	});
});

describe('DELETE /engagements/{id}', () => {
	it('deletes, audits, returns { deleted: true }', async () => {
		const res = await detail.DELETE(ev({}, { id: 'e1' }));
		expect(await res.json()).toEqual({ deleted: true });
		expect(mocks.recordAudit.mock.calls[0][3]).toMatchObject({ action: 'engagement_deleted' });
	});
	it('404s (skips audit) when nothing matched', async () => {
		mocks.deleteEngagement.mockResolvedValueOnce(false);
		expect((await detail.DELETE(ev({}, { id: 'e9' }))).status).toBe(404);
		expect(mocks.recordAudit).not.toHaveBeenCalled();
	});
});

describe('POST /engagements/{id}/bind', () => {
	it('binds, audits project_bound, returns { id, bound: true }', async () => {
		const res = await bind.POST(ev({ project_id: 'p1', name: 'One' }, { id: 'e1' }));
		expect(await res.json()).toEqual({ id: 'e1', bound: true });
		expect(mocks.bindEngagementProject.mock.calls[0][2]).toBe('e1');
		expect(mocks.recordAudit.mock.calls[0][3]).toMatchObject({ action: 'project_bound', engagementId: 'e1' });
	});
	it('maps a 400 from a bad body (no project_id) without auditing', async () => {
		mocks.parseBindProject.mockImplementationOnce(() => {
			throw new EngagementsError(400, 'project_id is required');
		});
		expect((await bind.POST(ev({}, { id: 'e1' }))).status).toBe(400);
		expect(mocks.recordAudit).not.toHaveBeenCalled();
	});
	it('maps a 404 when the engagement is missing', async () => {
		mocks.bindEngagementProject.mockRejectedValueOnce(new EngagementsError(404, 'no such engagement'));
		expect((await bind.POST(ev({ project_id: 'p1' }, { id: 'e9' }))).status).toBe(404);
	});
});
