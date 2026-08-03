// Route tests for POST /v1/you/dojos — self-serve create (F3a). Caller resolved
// from the JWT; 201 + the new dōjō on success; parse/collision errors → status;
// 401 propagates. Store mocked.
import { describe, it, expect, vi, beforeEach } from 'vitest';

class AdminError extends Error {
	constructor(
		readonly status: number,
		message: string
	) {
		super(message);
	}
}

const mocks = vi.hoisted(() => ({ resolveCaller: vi.fn(), createDojo: vi.fn(), parseNewDojo: vi.fn() }));

vi.mock('$lib/server/dojo-supabase', () => ({ dojoDb: () => ({}) }));
vi.mock('$lib/server/dojo-auth', () => ({
	resolveCaller: mocks.resolveCaller,
	apiError: (status: number, message: string) =>
		new Response(JSON.stringify({ error: message }), { status, headers: { 'content-type': 'application/json' } })
}));
vi.mock('$lib/server/admin-data', () => ({
	createDojo: mocks.createDojo,
	parseNewDojo: mocks.parseNewDojo,
	AdminError
}));

const route = await import('./+server');

function ev(body: unknown) {
	return {
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
	mocks.resolveCaller.mockClear().mockResolvedValue({ userId: 'u1', db: {} });
	mocks.parseNewDojo.mockClear().mockImplementation((b) => ({ name: b.name, kind: b.kind }));
	mocks.createDojo.mockClear().mockResolvedValue({ id: 't-new', key: 'org/acme', name: 'Acme' });
});

describe('POST /v1/you/dojos', () => {
	it('creates the dōjō for the caller and returns 201 + the row', async () => {
		const res = await route.POST(ev({ name: 'Acme', kind: 'employer' }));
		expect(res.status).toBe(201);
		expect(await res.json()).toEqual({ id: 't-new', key: 'org/acme', name: 'Acme' });
		expect(mocks.createDojo.mock.calls[0][1]).toBe('u1');
		expect(mocks.createDojo.mock.calls[0][2]).toEqual({ name: 'Acme', kind: 'employer' });
	});
	it('maps a parse 400 without creating', async () => {
		mocks.parseNewDojo.mockImplementationOnce(() => {
			throw new AdminError(400, 'name is required');
		});
		expect((await route.POST(ev({ kind: 'employer' }))).status).toBe(400);
		expect(mocks.createDojo).not.toHaveBeenCalled();
	});
	it('maps a key collision 409', async () => {
		mocks.createDojo.mockRejectedValueOnce(new AdminError(409, 'exists'));
		expect((await route.POST(ev({ name: 'Acme', kind: 'client' }))).status).toBe(409);
	});
	it('propagates a 401 without creating', async () => {
		mocks.resolveCaller.mockRejectedValueOnce(new Response('{}', { status: 401 }));
		expect((await route.POST(ev({ name: 'Acme', kind: 'client' }))).status).toBe(401);
		expect(mocks.createDojo).not.toHaveBeenCalled();
	});
});
