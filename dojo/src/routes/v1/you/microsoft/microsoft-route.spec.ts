// GET /v1/you/microsoft — whether the signed-in person's Microsoft account is
// linked (#308 R5c), for My dōjōs to say so when it could not be.
import { describe, it, expect, vi, beforeEach } from 'vitest';

const mocks = vi.hoisted(() => ({ resolveCaller: vi.fn(), microsoftLinkStatus: vi.fn() }));
const SYSTEM = vi.hoisted(() => ({ as: 'system' }));

vi.mock('$lib/server/dojo-supabase', () => ({ dojoDb: () => SYSTEM }));
vi.mock('$lib/server/dojo-auth', () => ({
	resolveCaller: mocks.resolveCaller,
	apiError: (status: number, message: string) =>
		new Response(JSON.stringify({ error: message }), { status, headers: { 'content-type': 'application/json' } })
}));
vi.mock('$lib/server/azure-signin', () => ({ microsoftLinkStatus: mocks.microsoftLinkStatus }));

const { GET } = await import('./+server');
const ev = () => ({ request: new Request('http://x/v1/you/microsoft'), locals: {} }) as never;
const MS = { state: 'ok', objectId: '00000000-0000-4000-8000-0000000000a1', name: null };

beforeEach(() => {
	mocks.resolveCaller.mockReset().mockResolvedValue({ userId: 'p1', authUserId: 'u1', email: null, accessToken: 'jwt', githubId: null, microsoft: MS });
	mocks.microsoftLinkStatus.mockReset().mockResolvedValue({ state: 'linked' });
});

describe('GET /v1/you/microsoft', () => {
	it('answers the link status of the caller`s own Microsoft account, by Supabase`s word', async () => {
		const res = await GET(ev());
		expect(res.status).toBe(200);
		expect(await res.json()).toEqual({ state: 'linked' });
		// The system reads who holds the identity: another person's row is what a
		// conflict is, and the caller cannot see it.
		expect(mocks.microsoftLinkStatus).toHaveBeenCalledWith(SYSTEM, 'p1', MS);
	});

	it('propagates the auth refusal', async () => {
		mocks.resolveCaller.mockRejectedValue(new Response('{}', { status: 401 }));
		expect((await GET(ev())).status).toBe(401);
		expect(mocks.microsoftLinkStatus).not.toHaveBeenCalled();
	});

	it('a failed read is a 500, never a status', async () => {
		class AdminError extends Error {
			status = 500;
		}
		mocks.microsoftLinkStatus.mockRejectedValue(Object.assign(new AdminError('boom'), { status: 500 }));
		const res = await GET(ev());
		expect(res.status).toBe(500);
	});
});
