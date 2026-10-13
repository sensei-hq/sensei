// Route-level tests for the Worker rules-federation endpoints (D1 tenant-path):
//   POST   /v1/t/{origin}/{org}/rules        → publish
//   GET    /v1/t/{origin}/{org}/rules?since= → pull deltas
//   DELETE /v1/t/{origin}/{org}/rules/{id}   → tombstone/retract
//
// The in-Worker port of dojo-mind's `publish_rule` / `pull_rules` /
// `retract_rule` (api.rs). Auth is `resolveTenantCaller`: the person's session
// (#236 step 4).
// This spec covers the HANDLER wiring (auth floor, body validation, error
// mapping, audit call, status codes) with the `rules-data` store fully mocked;
// the store's own SQL-shape logic is covered by `rules-data.spec.ts`. No live
// Worker/DB (mirrors `relay/push/push-routes.spec.ts`).
import { describe, it, expect, vi, beforeEach } from 'vitest';

// A member presenting their session (the default), and the same member with
// no session — refused since #236 step 4e.
const caller = {
	userId: '11111111-1111-1111-1111-111111111111',
	membershipId: 'mem-1',
	tenantId: 't1',
	role: 'contributor',
	access: 1,
	accessToken: 'jwt'
};
const { accessToken: _session, ...deviceCaller } = caller;
// `class RulesError` is hoisted so the mock factory (also hoisted) can reference
// it; the `vi.fn` mocks live in a `vi.hoisted` block for the same reason.
class RulesError extends Error {
	constructor(
		readonly status: number,
		message: string
	) {
		super(message);
	}
}

const mocks = vi.hoisted(() => ({
	resolveTenantCaller: vi.fn(),
	upsertRuleNamespace: vi.fn(),
	publishRule: vi.fn(),
	pullRules: vi.fn(),
	retractRule: vi.fn(),
	recordRulesAudit: vi.fn(),
	parsePublishedRule: vi.fn()
}));
const {
	resolveTenantCaller,
	upsertRuleNamespace,
	publishRule,
	pullRules,
	retractRule,
	recordRulesAudit,
	parsePublishedRule
} = mocks;

// Which client each store call got. The store is mocked, so they are markers.
const SESSION = vi.hoisted(() => ({ as: 'session' }));
const SYSTEM = vi.hoisted(() => ({ as: 'system' }));
vi.mock('$lib/server/dojo-supabase', () => ({
	dojoDb: () => SYSTEM,
	userDb: () => SESSION,
	ACCESS: { member: 0, contributor: 1, maintainer: 2, lead: 3, admin: 4 }
}));
vi.mock('$lib/server/dojo-auth', () => ({
	resolveTenantCaller: mocks.resolveTenantCaller,
	apiError: (status: number, message: string) =>
		new Response(JSON.stringify({ error: message }), { status, headers: { 'content-type': 'application/json' } }),
	ACCESS: { member: 0, contributor: 1, maintainer: 2, lead: 3, admin: 4 }
}));
vi.mock('$lib/server/rules-data', () => ({
	parsePublishedRule: mocks.parsePublishedRule,
	upsertRuleNamespace: mocks.upsertRuleNamespace,
	publishRule: mocks.publishRule,
	pullRules: mocks.pullRules,
	retractRule: mocks.retractRule,
	recordRulesAudit: mocks.recordRulesAudit,
	RulesError
}));

const { POST, GET } = await import('./+server');
const { DELETE } = await import('./[id]/+server');

function req(method: string, body?: unknown, url = 'http://x/'): Request {
	return new Request(url, {
		method,
		headers: { authorization: 'Bearer device-token-xyz' },
		body: body === undefined ? undefined : JSON.stringify(body)
	});
}
function ev(method: string, body?: unknown, opts?: { url?: string; id?: string }) {
	return {
		params: { origin: 'github', org: 'sensei-hq', id: opts?.id },
		request: req(method, body, opts?.url),
		locals: {},
		url: new URL(opts?.url ?? 'http://x/')
	} as never;
}

const RULE = {
	content_hash: 'a'.repeat(64),
	scope_key: 'organization',
	namespace_slug: 'sensei-hq',
	namespace_name: 'Sensei HQ',
	rule_type: 'convention',
	title: 'Always use TDD',
	content: 'Always use TDD.',
	impact: null,
	enforcement: 'mandatory',
	origin_repo: 'sensei/daemon',
	published_by: 'senseid',
	published_at: '1970-01-01T00:00:00Z'
};

beforeEach(() => {
	resolveTenantCaller.mockClear().mockResolvedValue(caller);
	upsertRuleNamespace.mockClear().mockResolvedValue('ns-1');
	publishRule.mockClear().mockResolvedValue({ id: 'rule-1', version: 1, seq: 7 });
	pullRules.mockClear().mockResolvedValue({ rules: [], cursor: 0 });
	retractRule.mockClear().mockResolvedValue(true);
	recordRulesAudit.mockClear();
	parsePublishedRule.mockClear().mockImplementation((body) =>
		typeof body.content_hash === 'string' && body.content_hash ? { ...body } : null
	);
});

describe('POST /v1/t/{origin}/{org}/rules — publish', () => {
	it('publishes and returns {id, version, seq} (dojo_protocol::PublishResponse)', async () => {
		const res = await POST(ev('POST', RULE));
		expect(res.status).toBe(200);
		expect(await res.json()).toEqual({ id: 'rule-1', version: 1, seq: 7 });
	});

	it('auths at the maintainer floor (access matrix: rules W = maintainer)', async () => {
		await POST(ev('POST', RULE));
		expect(resolveTenantCaller).toHaveBeenCalledTimes(1);
		const args = resolveTenantCaller.mock.calls[0];
		expect(args[0]).toBe('github'); // origin
		expect(args[1]).toBe('sensei-hq'); // org
		expect(args[3]).toBe(2); // ACCESS.maintainer
	});

	it('server-controls attribution: publishRule is called with the caller, not the body', async () => {
		await POST(ev('POST', { ...RULE, published_by: 'attacker' }));
		// publishRule(db, tenantId, rule, publishedBy, namespaceId).
		expect(publishRule.mock.calls[0][1]).toBe(caller.tenantId); // the caller's tenant owns it (#236)
		expect(publishRule.mock.calls[0][3]).toBe(caller.userId);
	});

	it('writes a publish audit event (tenant, action=publish, target=rule id)', async () => {
		await POST(ev('POST', RULE));
		expect(recordRulesAudit).toHaveBeenCalledTimes(1);
		const a = recordRulesAudit.mock.calls[0];
		expect(a[1]).toBe('t1'); // tenantId
		expect(a[2]).toBe('publish'); // action
		expect(a[3]).toBe('rule-1'); // target (the returned id)
		expect(a[4]).toBe(caller.userId); // actor
	});

	// #236 step 4: the rule is the person's (shared_rules_write_maintainer); the
	// namespace registry and the audit trail are the system's.
	it('with the person\'s session, files the rule as them under the namespace the system registered', async () => {
		await POST(ev('POST', RULE));
		expect(upsertRuleNamespace.mock.calls[0][0]).toBe(SYSTEM);
		expect(publishRule.mock.calls[0][0]).toBe(SESSION);
		expect(publishRule.mock.calls[0][4]).toBe('ns-1');
		expect(recordRulesAudit.mock.calls[0][0]).toBe(SYSTEM);
	});

	it('with no session is 401 — the service role never files the rule (#236 step 4e)', async () => {
		resolveTenantCaller.mockResolvedValueOnce(deviceCaller);
		const res = await POST(ev('POST', RULE));
		expect(res.status).toBe(401);
		expect(publishRule).not.toHaveBeenCalled();
	});

	it('400s a body missing required fields (parser returns null)', async () => {
		const res = await POST(ev('POST', { title: 'x' }));
		expect(res.status).toBe(400);
		expect(publishRule).not.toHaveBeenCalled();
	});

	it('maps a RulesError(500) from the store to a 500 (never silent)', async () => {
		publishRule.mockRejectedValueOnce(new RulesError(500, 'ns boom'));
		const res = await POST(ev('POST', RULE));
		expect(res.status).toBe(500);
		expect((await res.json()).error).toMatch(/ns boom/);
	});

	it('propagates a 401 Response thrown by resolveTenantCaller', async () => {
		resolveTenantCaller.mockRejectedValueOnce(new Response('{}', { status: 401 }));
		const res = await POST(ev('POST', RULE));
		expect(res.status).toBe(401);
	});

	it('propagates a 403 Response (insufficient device-token role)', async () => {
		resolveTenantCaller.mockRejectedValueOnce(new Response('{}', { status: 403 }));
		const res = await POST(ev('POST', RULE));
		expect(res.status).toBe(403);
	});
});

describe('GET /v1/t/{origin}/{org}/rules?since= — pull', () => {
	it('returns the store PullResponse verbatim', async () => {
		pullRules.mockResolvedValueOnce({
			rules: [{ id: 'r1', seq: 5, status: 'active', version: 1 }],
			cursor: 5
		} as never);
		const res = await GET(ev('GET', undefined, { url: 'http://x/?since=3' }));
		expect(res.status).toBe(200);
		expect(await res.json()).toEqual({ rules: [{ id: 'r1', seq: 5, status: 'active', version: 1 }], cursor: 5 });
	});

	it('parses ?since= and forwards it to the store', async () => {
		await GET(ev('GET', undefined, { url: 'http://x/?since=42' }));
		expect(pullRules.mock.calls[0][1]).toBe(caller.tenantId); // only the caller's tenant (#236)
		expect(pullRules.mock.calls[0][2]).toBe(42);
	});

	it('defaults since to 0 when the param is absent', async () => {
		await GET(ev('GET', undefined, { url: 'http://x/' }));
		expect(pullRules.mock.calls[0][2]).toBe(0);
	});

	it('defaults since to 0 when the param is non-numeric', async () => {
		await GET(ev('GET', undefined, { url: 'http://x/?since=abc' }));
		expect(pullRules.mock.calls[0][2]).toBe(0);
	});

	it('pull requires only the member floor', async () => {
		await GET(ev('GET', undefined, { url: 'http://x/' }));
		expect(resolveTenantCaller.mock.calls[0][3]).toBe(0); // ACCESS.member
	});

	it('with the person\'s session, pulls through dojo.tenant_shared_rules as them', async () => {
		await GET(ev('GET', undefined, { url: 'http://x/' }));
		expect(pullRules.mock.calls[0][0]).toBe(SESSION);
	});

	it('with no session is 401 — nothing is pulled (#236 step 4e)', async () => {
		resolveTenantCaller.mockResolvedValueOnce(deviceCaller);
		const res = await GET(ev('GET', undefined, { url: 'http://x/?since=4' }));
		expect(res.status).toBe(401);
		expect(pullRules).not.toHaveBeenCalled();
	});

	it('maps a store RulesError(500) to 500', async () => {
		pullRules.mockRejectedValueOnce(new RulesError(500, 'pull boom'));
		const res = await GET(ev('GET', undefined, { url: 'http://x/' }));
		expect(res.status).toBe(500);
	});

	it('propagates a 401 from auth', async () => {
		resolveTenantCaller.mockRejectedValueOnce(new Response('{}', { status: 401 }));
		const res = await GET(ev('GET', undefined, { url: 'http://x/' }));
		expect(res.status).toBe(401);
	});
});

describe('DELETE /v1/t/{origin}/{org}/rules/{id} — retract', () => {
	it('tombstones and returns 204', async () => {
		const res = await DELETE(ev('DELETE', undefined, { id: 'rule-1' }));
		expect(res.status).toBe(204);
		expect(retractRule.mock.calls[0][1]).toBe(caller.tenantId); // only the caller's tenant (#236)
		expect(retractRule.mock.calls[0][2]).toBe('rule-1');
	});

	it('writes a retract audit event', async () => {
		await DELETE(ev('DELETE', undefined, { id: 'rule-1' }));
		expect(recordRulesAudit).toHaveBeenCalledTimes(1);
		const a = recordRulesAudit.mock.calls[0];
		expect(a[2]).toBe('retract');
		expect(a[3]).toBe('rule-1');
	});

	it('with the person\'s session, retracts as them and audits as the system', async () => {
		await DELETE(ev('DELETE', undefined, { id: 'rule-1' }));
		expect(retractRule.mock.calls[0][0]).toBe(SESSION);
		expect(recordRulesAudit.mock.calls[0][0]).toBe(SYSTEM);
	});

	it('with no session is 401 — the service role never retracts (#236 step 4e)', async () => {
		resolveTenantCaller.mockResolvedValueOnce({ ...deviceCaller, role: 'maintainer', access: 2 });
		const res = await DELETE(ev('DELETE', undefined, { id: 'rule-1' }));
		expect(res.status).toBe(401);
		expect(retractRule).not.toHaveBeenCalled();
		expect(recordRulesAudit).not.toHaveBeenCalled();
	});

	it('404s when no active row matched (store returns false)', async () => {
		retractRule.mockResolvedValueOnce(false);
		const res = await DELETE(ev('DELETE', undefined, { id: 'rule-x' }));
		expect(res.status).toBe(404);
		expect(recordRulesAudit).not.toHaveBeenCalled();
	});

	it('requires the maintainer floor (access matrix: rules W = maintainer)', async () => {
		await DELETE(ev('DELETE', undefined, { id: 'r' }));
		expect(resolveTenantCaller.mock.calls[0][3]).toBe(2); // ACCESS.maintainer
	});

	it('maps a store RulesError(500) to 500', async () => {
		retractRule.mockRejectedValueOnce(new RulesError(500, 'retract boom'));
		const res = await DELETE(ev('DELETE', undefined, { id: 'rule-1' }));
		expect(res.status).toBe(500);
	});

	it('propagates a 403 from auth', async () => {
		resolveTenantCaller.mockRejectedValueOnce(new Response('{}', { status: 403 }));
		const res = await DELETE(ev('DELETE', undefined, { id: 'rule-1' }));
		expect(res.status).toBe(403);
	});
});
