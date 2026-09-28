// Route tests for PATCH /v1/you/metrics/activation.
//
// The wiring, not the activation logic (that is metric-activation.spec.ts): that
// the caller's PRINCIPAL is what the ruling is keyed on, that a bad body is
// refused before any write, and that a refusal keeps its status instead of
// collapsing into a 500.
import { describe, it, expect, vi, beforeEach } from 'vitest';

class AdminError extends Error {
	constructor(
		readonly status: number,
		message: string
	) {
		super(message);
	}
}

const mocks = vi.hoisted(() => ({
	resolveCaller: vi.fn(),
	setMetricActivation: vi.fn()
}));

vi.mock('$lib/server/dojo-supabase', () => ({ dojoDb: () => ({}) }));
vi.mock('$lib/server/dojo-auth', () => ({
	resolveCaller: mocks.resolveCaller,
	apiError: (status: number, message: string) =>
		new Response(JSON.stringify({ error: message }), {
			status,
			headers: { 'content-type': 'application/json' }
		})
}));
vi.mock('$lib/server/admin-data', () => ({ AdminError }));
vi.mock('$lib/server/metric-activation', () => ({
	setMetricActivation: mocks.setMetricActivation
}));

const route = await import('./+server');

const DB = { marker: 'db' };
const OUT = {
	repo_key: 'github.com/acme/api',
	metric: 'ftr',
	enabled: false,
	tenant: 'organization/acme'
};

function patch(body: unknown) {
	return route.PATCH({
		request: new Request('http://x/v1/you/metrics/activation', {
			method: 'PATCH',
			body: JSON.stringify(body),
			headers: { 'content-type': 'application/json' }
		}),
		locals: {}
	} as never);
}

beforeEach(() => {
	vi.clearAllMocks();
	mocks.resolveCaller.mockResolvedValue({ userId: 'p-alice', db: DB });
	mocks.setMetricActivation.mockResolvedValue(OUT);
});

describe('PATCH /v1/you/metrics/activation', () => {
	it('rules on behalf of the resolved PRINCIPAL, not anything in the body', async () => {
		// The body cannot name the principal or the tenant. If it could, a member
		// of one dōjō could switch off a metric another dōjō is paying for.
		const res = await patch({
			repo_key: 'github.com/acme/api',
			metric: 'ftr',
			enabled: false,
			principal_id: 'p-mallory',
			tenant_id: 't-other'
		});
		expect(res.status).toBe(200);
		expect(mocks.setMetricActivation).toHaveBeenCalledWith(
			DB,
			'p-alice',
			'github.com/acme/api',
			'ftr',
			false
		);
		expect(await res.json()).toEqual(OUT);
	});

	it('passes enabled=true through rather than treating it as absent', async () => {
		// `if (!body.enabled)` would make turning a metric back ON
		// indistinguishable from omitting the field — a 400 on the one operation a
		// user most needs to work after switching something off by mistake.
		await patch({ repo_key: 'github.com/acme/api', metric: 'ftr', enabled: true });
		expect(mocks.setMetricActivation).toHaveBeenCalledWith(
			DB,
			'p-alice',
			'github.com/acme/api',
			'ftr',
			true
		);
	});

	it('refuses a non-boolean `enabled` instead of coercing it', async () => {
		// "false" is truthy. Coercion here would turn a metric ON for a client that
		// asked to turn it off — silently resuming work the tenant pays to avoid.
		const res = await patch({ repo_key: 'github.com/acme/api', metric: 'ftr', enabled: 'false' });
		expect(res.status).toBe(400);
		expect(mocks.setMetricActivation).not.toHaveBeenCalled();
	});

	it.each([
		['repo_key', { metric: 'ftr', enabled: false }],
		['metric', { repo_key: 'github.com/acme/api', enabled: false }]
	])('refuses a missing %s before writing anything', async (_name, body) => {
		const res = await patch(body);
		expect(res.status).toBe(400);
		expect(mocks.setMetricActivation).not.toHaveBeenCalled();
	});

	it('requires a caller before writing anything', async () => {
		mocks.resolveCaller.mockRejectedValue(
			new Response(JSON.stringify({ error: 'unauthenticated' }), { status: 401 })
		);
		const res = await patch({ repo_key: 'github.com/acme/api', metric: 'ftr', enabled: false });
		expect(res.status).toBe(401);
		expect(mocks.setMetricActivation).not.toHaveBeenCalled();
	});

	it.each([
		[403, 'you may not change metric activation'],
		[404, 'no repository for this account'],
		[404, 'unknown metric zzz']
	])('keeps a %i refusal as %i rather than collapsing it to 500', async (status, message) => {
		mocks.setMetricActivation.mockRejectedValue(new AdminError(status, message));
		const res = await patch({ repo_key: 'github.com/acme/api', metric: 'ftr', enabled: false });
		expect(res.status).toBe(status);
		expect((await res.json()).error).toBe(message);
	});
});
