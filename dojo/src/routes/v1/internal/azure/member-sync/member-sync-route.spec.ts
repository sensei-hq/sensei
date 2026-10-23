// POST /v1/internal/azure/member-sync (#308 R5b, D44): the scheduled member
// read's trigger. The Worker's cron (worker-entry.js) calls it in-process; it is
// NEVER public — it answers only a request carrying the server secret
// (DOJO_CRON_SECRET) as a bearer, compared in constant time. Without the secret
// configured it is a 503, never open. The job itself is azure-member-sync.spec.ts.
import { describe, it, expect, vi, beforeEach } from 'vitest';

const mocks = vi.hoisted(() => ({ runAzureMemberSync: vi.fn(), env: {} as Record<string, string | undefined> }));

vi.mock('$env/dynamic/private', () => ({ env: mocks.env }));
vi.mock('$lib/server/dojo-supabase', () => ({ dojoDb: () => ({ client: 'service' }), serverSecret: () => 's' }));
vi.mock('$lib/server/azure-member-sync', () => ({ runAzureMemberSync: mocks.runAzureMemberSync }));

const route = await import('./+server');

const post = (auth?: string) =>
	route.POST({
		request: new Request('https://dojo.example.com/v1/internal/azure/member-sync', {
			method: 'POST',
			headers: auth ? { authorization: auth } : {}
		})
	} as never);

beforeEach(() => {
	mocks.runAzureMemberSync.mockReset();
	mocks.runAzureMemberSync.mockResolvedValue({ read: 1, results: [{ organization: 'example-org', members: { complete: true, listed: 3, ended: 1 } }] });
	for (const k of Object.keys(mocks.env)) delete mocks.env[k];
	Object.assign(mocks.env, {
		DOJO_CRON_SECRET: 'cron-secret-0000-0000-0000-0000',
		AZURE_DEVOPS_CLIENT_ID: 'client-id-0000',
		AZURE_DEVOPS_CLIENT_SECRET: 'client-secret-0000'
	});
});

describe('the member-sync trigger', () => {
	it('runs the scheduled read for the server secret, and answers what it came to', async () => {
		const res = await post('Bearer cron-secret-0000-0000-0000-0000');
		expect(res.status).toBe(200);
		expect(await res.json()).toEqual({ read: 1, results: [{ organization: 'example-org', members: { complete: true, listed: 3, ended: 1 } }] });
		const [db, deps] = mocks.runAzureMemberSync.mock.calls[0];
		expect(db).toEqual({ client: 'service' });
		expect(deps.cfg).toEqual({ clientId: 'client-id-0000', clientSecret: 'client-secret-0000' });
		expect(deps.adapterFor('example-org').organization).toBe('example-org');
	});

	it('refuses anything else (401) and runs nothing', async () => {
		for (const auth of [undefined, 'Bearer wrong', 'cron-secret-0000-0000-0000-0000', 'Bearer cron-secret-0000-0000-0000-000', 'Basic x']) {
			expect((await post(auth)).status).toBe(401);
		}
		expect(mocks.runAzureMemberSync).not.toHaveBeenCalled();
	});

	it('is a 503 — never open — when no secret or no app registration is configured', async () => {
		delete mocks.env.DOJO_CRON_SECRET;
		expect((await post('Bearer ')).status).toBe(503);
		expect((await post()).status).toBe(503);
		mocks.env.DOJO_CRON_SECRET = 'cron-secret-0000-0000-0000-0000';
		delete mocks.env.AZURE_DEVOPS_CLIENT_SECRET;
		expect((await post('Bearer cron-secret-0000-0000-0000-0000')).status).toBe(503);
		expect(mocks.runAzureMemberSync).not.toHaveBeenCalled();
	});

	it('a run that cannot read the connections is an error answer, not a success', async () => {
		mocks.runAzureMemberSync.mockRejectedValue(Object.assign(new Error('boom'), { status: 500 }));
		expect((await post('Bearer cron-secret-0000-0000-0000-0000')).status).toBe(500);
	});
});
