// Route specs for POST/GET /v1/t/{origin}/{org}/artifacts — the in-Worker
// artifact federation. Verifies the auth floors (POST=contributor, GET=member,
// device-token plane), that publish wires into the inline promote step, that a
// promote failure does NOT fail the already-committed contribution (logged, not
// thrown — mirrors dojo-mind), and the pull passes the tenant + cursor through
// and returns the ArtifactPullResponse shape. The `$lib/server` deps
// (dojo-supabase / dojo-auth / artifacts-data) are mocked — the store logic has
// its own unit specs (`artifacts-data.spec.ts`); this proves the wiring.
import { describe, it, expect, vi, beforeEach } from 'vitest';

// A shared spy set the mocks close over, reset per test.
const spies = {
	resolveApiKeyAccess: vi.fn(),
	publishArtifact: vi.fn(),
	pullArtifactsSince: vi.fn(),
	promoteCluster: vi.fn(),
	parsePublishedArtifact: vi.fn()
};

vi.mock('$lib/server/dojo-supabase', () => ({
	dojoDb: () => ({}) // the store fns are mocked, so the client is opaque here
}));

vi.mock('$lib/server/dojo-auth', () => ({
	ACCESS: { member: 0, contributor: 1, lead: 2, maintainer: 3, admin: 4 },
	apiError: (status: number, message: string) =>
		new Response(JSON.stringify({ error: message }), {
			status,
			headers: { 'content-type': 'application/json' }
		}),
	resolveApiKeyAccess: (...args: unknown[]) => spies.resolveApiKeyAccess(...args)
}));

vi.mock('$lib/server/artifacts-data', async () => {
	const actual = await vi.importActual<typeof import('$lib/server/artifacts-data')>(
		'$lib/server/artifacts-data'
	);
	return {
		...actual,
		publishArtifact: (...a: unknown[]) => spies.publishArtifact(...a),
		pullArtifactsSince: (...a: unknown[]) => spies.pullArtifactsSince(...a),
		promoteCluster: (...a: unknown[]) => spies.promoteCluster(...a),
		parsePublishedArtifact: (...a: unknown[]) => spies.parsePublishedArtifact(...a)
	};
});

const { POST, GET } = await import('./+server');

const CALLER = { tenantId: 't1', userId: 'user-uuid', role: 'contributor', access: 1, membershipId: 'm1' };

function req(body?: unknown): Request {
	return new Request('https://dojo.sensei-hq.com/v1/t/github/acme/artifacts', {
		method: 'POST',
		headers: { authorization: 'Bearer device-token', 'content-type': 'application/json' },
		body: body === undefined ? undefined : JSON.stringify(body)
	});
}

// A parsed artifact stand-in (parsePublishedArtifact is mocked to return this).
const ARTIFACT = { signature: 'a'.repeat(64), tenant_key: 'github/acme', kind: 'pattern' };

// Minimal SvelteKit RequestEvent shape the handlers read (params/request/url).
const evt = (request: Request, sinceQuery?: string) =>
	({
		params: { origin: 'github', org: 'acme' },
		request,
		url: new URL(
			`https://dojo.sensei-hq.com/v1/t/github/acme/artifacts${sinceQuery ? `?since=${sinceQuery}` : ''}`
		)
	}) as unknown as Parameters<typeof POST>[0];

beforeEach(() => {
	for (const s of Object.values(spies)) s.mockReset();
	spies.resolveApiKeyAccess.mockResolvedValue(CALLER);
	spies.parsePublishedArtifact.mockReturnValue(ARTIFACT);
	spies.publishArtifact.mockResolvedValue({ id: 'art-1', seq: 7 });
	spies.promoteCluster.mockResolvedValue({ outcome: 'queued', reason: 'below_score_bar' });
	spies.pullArtifactsSince.mockResolvedValue({ artifacts: [], cursor: 0 });
});

describe('POST /artifacts (contribute + promote)', () => {
	it('enforces the contributor floor on the device-token plane', async () => {
		await POST(evt(req(ARTIFACT)));
		expect(spies.resolveApiKeyAccess).toHaveBeenCalledWith(
			'github',
			'acme',
			expect.any(Request),
			1 // ACCESS.contributor
		);
	});

	it('publishes with the caller as attribution, then runs promoteCluster, returning {id,seq}', async () => {
		const res = await POST(evt(req(ARTIFACT)));
		expect(await res.json()).toEqual({ id: 'art-1', seq: 7 });
		// Attribution (contributed_by) is the authenticated caller, never the body.
		expect(spies.publishArtifact).toHaveBeenCalledWith(expect.anything(), 't1', ARTIFACT, 'user-uuid');
		// Promotion is triggered on the artifact's signature.
		expect(spies.promoteCluster).toHaveBeenCalledWith(expect.anything(), 't1', 'a'.repeat(64));
	});

	it('400s when the body fails validation / kind mismatch (parse → null)', async () => {
		spies.parsePublishedArtifact.mockReturnValue(null);
		const res = await POST(evt(req({ bogus: true })));
		expect(res.status).toBe(400);
		expect(spies.publishArtifact).not.toHaveBeenCalled();
		expect(spies.promoteCluster).not.toHaveBeenCalled();
	});

	it('does NOT fail the committed publish when promotion throws (logged, not thrown)', async () => {
		spies.promoteCluster.mockRejectedValue(new Error('promote boom'));
		const errSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
		const res = await POST(evt(req(ARTIFACT)));
		// The publish response is still returned 200 — promotion is best-effort.
		expect(res.status).toBe(200);
		expect(await res.json()).toEqual({ id: 'art-1', seq: 7 });
		expect(errSpy).toHaveBeenCalledWith(expect.stringContaining('promote boom'));
		errSpy.mockRestore();
	});

	it('propagates the auth Response (e.g. 403) without publishing', async () => {
		spies.resolveApiKeyAccess.mockRejectedValue(
			new Response(JSON.stringify({ error: 'contributor role required' }), { status: 403 })
		);
		const res = await POST(evt(req(ARTIFACT)));
		expect(res.status).toBe(403);
		expect(spies.publishArtifact).not.toHaveBeenCalled();
	});
});

describe('GET /artifacts (pull)', () => {
	it('enforces the member floor on the device-token plane', async () => {
		await GET(evt(req(), '5'));
		expect(spies.resolveApiKeyAccess).toHaveBeenCalledWith('github', 'acme', expect.any(Request), 0);
	});

	it('passes the tenant + parsed since cursor and returns the pull shape', async () => {
		spies.pullArtifactsSince.mockResolvedValue({
			artifacts: [{ id: 'art-9', seq: 9, status: 'published' }],
			cursor: 9
		});
		const res = await GET(evt(req(), '5'));
		expect(spies.pullArtifactsSince).toHaveBeenCalledWith(expect.anything(), 't1', 5);
		expect(await res.json()).toEqual({
			artifacts: [{ id: 'art-9', seq: 9, status: 'published' }],
			cursor: 9
		});
	});

	it('defaults since to 0 when absent or non-numeric', async () => {
		await GET(evt(req()));
		expect(spies.pullArtifactsSince).toHaveBeenCalledWith(expect.anything(), 't1', 0);
		spies.pullArtifactsSince.mockClear();
		await GET(evt(req(), 'notanumber'));
		expect(spies.pullArtifactsSince).toHaveBeenCalledWith(expect.anything(), 't1', 0);
	});

	it('propagates the auth Response (e.g. 401) without pulling', async () => {
		spies.resolveApiKeyAccess.mockRejectedValue(
			new Response(JSON.stringify({ error: 'unauthenticated' }), { status: 401 })
		);
		const res = await GET(evt(req()));
		expect(res.status).toBe(401);
		expect(spies.pullArtifactsSince).not.toHaveBeenCalled();
	});
});
