// The daemon's relay calls run AS THE PERSON (#236 step 4,
// docs/spec/dojo/access-matrix.md).
//
// POST session / segments / inbox and GET inbox resolve the caller through
// resolveTenantCaller: the relay rows (and the run's workspace) are read and
// written with the person's JWT, so the policies in database/policies/dojo/
// decide. A caller with no session is refused — the service role never stands
// in. The seat stays the system's (billing bookkeeping).
import { describe, it, expect, vi, beforeEach } from 'vitest';

type Terminal = { data: unknown; error: unknown };

// A chainable, awaitable supabase-js stub that records every table it touched.
function makeClient() {
	const tables: string[] = [];
	let queue: Terminal[] = [];
	const next = () => Promise.resolve(queue.shift() ?? { data: null, error: null });
	const b: Record<string, unknown> = {};
	b.from = (t: string) => {
		tables.push(t);
		return b;
	};
	for (const op of ['select', 'eq', 'in', 'gt', 'order', 'update', 'insert', 'upsert', 'is']) b[op] = () => b;
	b.rpc = (fn: string) => {
		tables.push(`rpc:${fn}`);
		return next();
	};
	b.maybeSingle = next;
	b.single = next;
	b.then = (ok: (v: Terminal) => unknown, err: (e: unknown) => unknown) => next().then(ok, err);
	return {
		builder: b,
		tables,
		queue: (...r: Terminal[]) => (queue = r),
		reset: () => {
			tables.length = 0;
			queue = [];
		}
	};
}

const session = makeClient();
const system = makeClient();
const sessionTokens: string[] = [];

vi.mock('$lib/server/dojo-supabase', () => ({
	dojoDb: () => system.builder,
	userDb: (token: string) => {
		sessionTokens.push(token);
		return session.builder;
	},
	ACCESS: { member: 0, contributor: 1, maintainer: 2, lead: 3, admin: 4 }
}));

const base = { tenantId: 't1', userId: 'p1', membershipId: 'mem-1', role: 'contributor', access: 1 };
let caller: typeof base & { accessToken?: string } = base;
const resolveTenantCaller = vi.fn(async () => caller);
vi.mock('$lib/server/dojo-auth', () => ({
	resolveTenantCaller,
	resolveTenantAccess: vi.fn(async () => caller),
	apiError: (status: number, message: string) =>
		new Response(JSON.stringify({ error: message }), { status, headers: { 'content-type': 'application/json' } }),
	ACCESS: { member: 0, contributor: 1, maintainer: 2, lead: 3, admin: 4 }
}));
vi.mock('$lib/server/relay-push-env', () => ({
	sendRelayPushFromEnv: async () => ({ gated: true, attempted: 0, delivered: 0, disabled: [] })
}));

const { POST: sessionPOST } = await import('./session/+server');
const { POST: segmentsPOST } = await import('./segments/+server');
const { POST: inboxPOST, GET: inboxGET } = await import('./inbox/+server');

function ev(opts: { body?: unknown; query?: string; platform?: unknown } = {}) {
	const url = new URL(`http://x/${opts.query ? `?${opts.query}` : ''}`);
	const request = new Request(url, opts.body === undefined ? {} : { method: 'POST', body: JSON.stringify(opts.body) });
	return { params: { origin: 'organization', org: 'acme' }, request, locals: {}, url, platform: opts.platform } as never;
}

const routes: Array<{ name: string; run: () => Promise<Response>; queue: Terminal[] }> = [
	{
		name: 'session POST',
		run: async () => sessionPOST(ev({ body: { run_id: 'run-1', status: 'running' } })),
		queue: [{ data: null, error: null }, { data: { id: 's1' }, error: null }]
	},
	{
		name: 'segments POST',
		run: async () => segmentsPOST(ev({ body: { run_id: 'run-1', segments: [{ seq: 0, title: 'P1' }] } })),
		queue: [{ data: { id: 's1' }, error: null }, { data: null, error: null }]
	},
	{
		name: 'inbox POST',
		run: async () => inboxPOST(ev({ body: { run_id: 'run-1', kind: 'chat', payload: {} } })),
		queue: [{ data: { id: 's1', title: 'Run' }, error: null }, { data: { id: 'i1', seq: 1 }, error: null }]
	},
	{
		name: 'inbox GET',
		run: async () => inboxGET(ev({ query: 'since=0' })),
		queue: [{ data: [], error: null }]
	}
];

beforeEach(() => {
	session.reset();
	system.reset();
	sessionTokens.length = 0;
	resolveTenantCaller.mockClear();
});

describe('the daemon’s relay calls run as the person (#236 step 4)', () => {
	for (const r of routes) {
		it(`${r.name} resolves through resolveTenantCaller at the contributor floor`, async () => {
			caller = { ...base, accessToken: 'jwt' };
			session.queue(...r.queue);
			const res = await r.run();
			expect(res.status).toBe(200);
			expect(resolveTenantCaller).toHaveBeenCalledWith('organization', 'acme', expect.any(Request), 1);
		});

		it(`${r.name} with the person’s session reads and writes as them`, async () => {
			caller = { ...base, accessToken: 'jwt' };
			session.queue(...r.queue);
			const res = await r.run();
			expect(res.status).toBe(200);
			expect(sessionTokens).toContain('jwt');
			expect(session.tables.filter((t) => t.startsWith('relay_')).length).toBeGreaterThan(0);
			expect(system.tables.filter((t) => t.startsWith('relay_'))).toEqual([]);
		});

		it(`${r.name} with no session is 401 — the service role never stands in (#236 step 4e)`, async () => {
			caller = base;
			system.queue(...r.queue);
			const res = await r.run();
			expect(res.status).toBe(401);
			expect(sessionTokens).toEqual([]);
			expect(system.tables.filter((t) => t.startsWith('relay_'))).toEqual([]);
		});
	}

	it('session POST attaches the run to its shared workspace as the person, and keeps the seat the system’s', async () => {
		// #299: a run never creates a workspace — it attaches to the shared one the
		// sync filed, through attach_run_to_workspace, as the person.
		caller = { ...base, accessToken: 'jwt' };
		session.queue({ data: null, error: null }, { data: { id: 's1' }, error: null }, { data: 'proj-1', error: null });
		system.queue({ data: null, error: null });
		const scheduled: Promise<unknown>[] = [];
		const platform = { context: { waitUntil: (p: Promise<unknown>) => scheduled.push(p) } };
		const res = await sessionPOST(
			ev({
				body: { run_id: 'run-1', project_slug: 'acme-api', workspace: { slug: 'acme/api', name: 'api' } },
				platform
			})
		);
		await Promise.all(scheduled);
		expect(res.status).toBe(200);
		expect(session.tables).toContain('rpc:attach_run_to_workspace');
		expect(session.tables).not.toContain('workspaces');
		expect(system.tables).not.toContain('rpc:attach_run_to_workspace');
		// #252: the run is then filed under that workspace, as the person.
		expect(session.tables.slice(session.tables.indexOf('rpc:attach_run_to_workspace'))).toContain('relay_sessions');
		expect(system.tables).toContain('namespaces');
		expect(session.tables).not.toContain('namespaces');
	});

	it('session POST files no workspace when the dōjō has none yet', async () => {
		caller = { ...base, accessToken: 'jwt' };
		session.queue({ data: null, error: null }, { data: { id: 's1' }, error: null }, { data: null, error: null });
		system.queue({ data: null, error: null });
		const scheduled: Promise<unknown>[] = [];
		const platform = { context: { waitUntil: (p: Promise<unknown>) => scheduled.push(p) } };
		await sessionPOST(ev({ body: { run_id: 'run-1', workspace: { slug: 'acme/api', name: 'api' } }, platform }));
		await Promise.all(scheduled);
		const after = session.tables.slice(session.tables.indexOf('rpc:attach_run_to_workspace') + 1);
		expect(after).not.toContain('relay_sessions');
	});
});
