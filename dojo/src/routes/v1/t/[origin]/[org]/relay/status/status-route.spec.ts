// GET …/relay/status — THAT runs are going, for the people the access matrix
// lets know it (#252): status only, through dojo.peer_run_status as the caller.
import { describe, it, expect, vi, beforeEach } from 'vitest';

const rpc = vi.fn();
const resolveTenantAccess = vi.fn();
vi.mock('$lib/server/dojo-auth', () => ({
	resolveTenantAccess,
	apiError: (status: number, message: string) =>
		new Response(JSON.stringify({ error: message }), { status, headers: { 'content-type': 'application/json' } }),
	ACCESS: { member: 0, contributor: 1, maintainer: 2, lead: 3, admin: 4 }
}));
vi.mock('$lib/server/caller-db', () => ({ callerDb: () => ({ rpc }) }));

const { GET } = await import('./+server');
const ev = () =>
	({ params: { origin: 'organization', org: 'acme' }, request: new Request('http://x/'), locals: {} }) as never;

beforeEach(() => {
	resolveTenantAccess.mockReset().mockResolvedValue({ tenantId: 't1', userId: 'p1', access: 0, accessToken: 'jwt' });
	rpc.mockReset();
});

describe('GET …/relay/status', () => {
	it('returns the runs the caller may know of, at the member floor', async () => {
		const runs = [{ run_id: 'r1', project_id: 'p', status: 'running', started_at: 'x', heartbeat_at: null, completed_at: null }];
		rpc.mockResolvedValue({ data: runs, error: null });
		const res = await GET(ev());
		expect(res.status).toBe(200);
		expect(await res.json()).toEqual({ runs });
		expect(resolveTenantAccess.mock.calls[0][4]).toBe(0);
		expect(rpc).toHaveBeenCalledWith('peer_run_status', { p_tenant: 't1' });
	});

	it('a failed read is a 500, never an empty list', async () => {
		rpc.mockResolvedValue({ data: null, error: { message: 'boom' } });
		expect((await GET(ev())).status).toBe(500);
	});

	it('propagates the auth refusal', async () => {
		resolveTenantAccess.mockRejectedValue(new Response('{}', { status: 403 }));
		expect((await GET(ev())).status).toBe(403);
	});
});
