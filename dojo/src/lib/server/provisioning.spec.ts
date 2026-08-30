// ensureProvisioned — the operation that closes the hole this whole slice is
// about: nothing in the dōjō created a tenant. Spec §II.7, §VIII.7 item 2.
//
// These run against `fakeDojoDb`, which actually stores rows and enforces the
// real unique constraints, because the headline property is IDEMPOTENCE — and a
// queue stub that replays canned answers would pass just as happily for an
// implementation that inserted a duplicate tenant on every sign-in.
import { describe, it, expect, beforeEach } from 'vitest';
import { fakeDojoDb, resetFakeIds, type FakeTable } from './fake-dojo-db';
import {
	ensureProvisioned,
	provisionWithToken,
	refreshForgeVisibility,
	MAX_VISIBILITY_REFRESH_PER_PASS
} from './provisioning';
import type { ForgeFacts } from './forge-github';

const PRINCIPAL = 'p-alice';

/** The dōjō tables provisioning touches, with the constraints that actually
 *  exist in the DDL — those are what make the idempotence assertions real. */
function tables(seed: Partial<Record<string, FakeTable>> = {}): Record<string, FakeTable> {
	return {
		identities: {
			rows: [],
			uniques: [{ columns: ['provider', 'subject'] }],
			...seed.identities
		},
		tenants: {
			rows: [],
			uniques: [{ columns: ['key'] }],
			...seed.tenants
		},
		tenant_connections: {
			rows: [],
			uniques: [
				{ columns: ['provider', 'external_id'], where: (r) => r.external_id != null },
				{
					columns: ['provider', 'external_slug'],
					lower: ['external_slug'],
					where: (r) => r.external_id == null
				}
			],
			...seed.tenant_connections
		},
		memberships: {
			rows: [],
			uniques: [{ columns: ['tenant_id', 'user_id'] }],
			...seed.memberships
		},
		repositories: {
			rows: [],
			// The constraint the whole scoping argument rests on: the SAME repository
			// legitimately exists under two tenants, so `repo_key` alone does not
			// identify a row (§8a "the sign-in refresh must be scoped").
			uniques: [{ columns: ['tenant_id', 'repo_key'] }],
			...seed.repositories
		}
	};
}

const FACTS: ForgeFacts = {
	provider: 'github',
	user: { id: '4242', login: 'jerrythomas', name: 'Jerry Thomas', email: 'j@example.com' },
	orgs: [
		{ id: '11', login: 'sensei-hq', role: 'admin' },
		{ id: '22', login: 'acme', role: 'member' }
	]
};

beforeEach(() => resetFakeIds());

describe('ensureProvisioned — the personal dōjō', () => {
	it('creates a personal tenant keyed personal/{login}, with the caller as admin', async () => {
		// D1: every authenticated user has a personal dōjō, immediately, with no
		// activation step. This is the row that did not exist anywhere before.
		const db = fakeDojoDb(tables());
		const out = await ensureProvisioned(db as never, PRINCIPAL, FACTS);

		expect(out.personal).toMatchObject({ key: 'personal/jerrythomas', origin: 'personal' });
		const tenant = db.tables.tenants.rows.find((t) => t.key === 'personal/jerrythomas');
		expect(tenant).toMatchObject({ origin: 'personal', slug: 'jerrythomas' });

		const membership = db.tables.memberships.rows.find((m) => m.tenant_id === tenant?.id);
		expect(membership).toMatchObject({ user_id: PRINCIPAL, role: 'admin', kind: 'personal' });
	});

	it('does not create a second personal tenant on a repeat pass', async () => {
		// The property a canned-answer stub cannot show. Two sign-ins, one tenant.
		const db = fakeDojoDb(tables());
		await ensureProvisioned(db as never, PRINCIPAL, FACTS);
		await ensureProvisioned(db as never, PRINCIPAL, FACTS);
		expect(db.tables.tenants.rows.filter((t) => t.origin === 'personal')).toHaveLength(1);
		expect(db.tables.memberships.rows.filter((m) => m.kind === 'personal')).toHaveLength(1);
	});

	it('keeps the existing personal tenant even when the forge login changed', async () => {
		// A GitHub handle is renameable. Re-deriving the slug and creating a
		// second personal dōjō would strand the first one's history, so the
		// lookup is by the caller's existing personal membership, not by slug.
		const db = fakeDojoDb(tables());
		await ensureProvisioned(db as never, PRINCIPAL, FACTS);
		const renamed = { ...FACTS, user: { ...FACTS.user, login: 'jerry-new' } };
		const out = await ensureProvisioned(db as never, PRINCIPAL, renamed);

		expect(out.personal?.key).toBe('personal/jerrythomas');
		expect(db.tables.tenants.rows.filter((t) => t.origin === 'personal')).toHaveLength(1);
	});

	it('picks a free slug when another user already holds personal/{login}', async () => {
		// Two different humans can be `jerrythomas` on two forges. `tenants.key`
		// is unique, so the second must land somewhere — not fail, and not join
		// the first person's dōjō.
		const db = fakeDojoDb(
			tables({
				tenants: {
					rows: [
						{
							id: 't-taken',
							key: 'personal/jerrythomas',
							origin: 'personal',
							slug: 'jerrythomas',
							name: 'Someone else'
						}
					],
					uniques: [{ columns: ['key'] }]
				}
			})
		);
		const out = await ensureProvisioned(db as never, 'p-other', FACTS);
		expect(out.personal?.key).not.toBe('personal/jerrythomas');
		expect(out.personal?.key).toMatch(/^personal\/jerrythomas-\d+$/);
		// and it is NOT the other person's tenant
		expect(out.personal?.id).not.toBe('t-taken');
	});
});

describe('ensureProvisioned — org tenants and connections', () => {
	it('creates one organization tenant per forge org, each with a verified connection', async () => {
		const db = fakeDojoDb(tables());
		const out = await ensureProvisioned(db as never, PRINCIPAL, FACTS);

		expect(out.tenants.map((t) => t.key).sort()).toEqual([
			'organization/acme',
			'organization/sensei-hq'
		]);

		const conn = db.tables.tenant_connections.rows.find((c) => c.external_slug === 'sensei-hq');
		expect(conn).toMatchObject({
			provider: 'github',
			external_id: '11', // the STABLE id, not the slug (§II.2)
			external_slug: 'sensei-hq',
			connected_by: PRINCIPAL
		});
		// We just proved control by reading the forge with the user's own token,
		// so the connection is verified — an unverified one confers nothing.
		expect(conn?.verified_at).toBeTruthy();
	});

	it('derives the membership role from the forge role', async () => {
		// owner/admin on the forge → tenant admin; plain member → contributor.
		const db = fakeDojoDb(tables());
		const out = await ensureProvisioned(db as never, PRINCIPAL, FACTS);
		expect(out.tenants.find((t) => t.key === 'organization/sensei-hq')?.role).toBe('admin');
		expect(out.tenants.find((t) => t.key === 'organization/acme')?.role).toBe('contributor');
	});

	it('joins the EXISTING tenant when the forge org is already connected', async () => {
		// The anti-duplication rule: one proven forge org maps to at most one
		// tenant, forever. A second user signing in must join, not fork it.
		const db = fakeDojoDb(tables());
		await ensureProvisioned(db as never, PRINCIPAL, FACTS);
		const before = db.tables.tenants.rows.length;

		// Bob is a DIFFERENT forge account in the same orgs — reusing Alice's
		// would (correctly) trip the one-account-one-person guard below.
		const bobFacts: ForgeFacts = {
			...FACTS,
			user: { id: '9999', login: 'bob', name: 'Bob', email: 'bob@example.com' }
		};
		await ensureProvisioned(db as never, 'p-bob', bobFacts);
		expect(db.tables.tenants.rows.filter((t) => t.origin === 'organization')).toHaveLength(2);
		// only Bob's personal tenant is new
		expect(db.tables.tenants.rows.length).toBe(before + 1);
		// both are members of sensei-hq
		const senseiHq = db.tables.tenants.rows.find((t) => t.key === 'organization/sensei-hq');
		expect(
			db.tables.memberships.rows.filter((m) => m.tenant_id === senseiHq?.id).map((m) => m.user_id).sort()
		).toEqual(['p-alice', 'p-bob']);
	});

	it('matches on the stable external_id, not the slug, when an org is renamed', async () => {
		// A renamed org must resolve to the SAME tenant. Matching on the slug
		// would fork the tenant on rename — and would let whoever claims the
		// freed name inherit its governance (§II.2).
		const db = fakeDojoDb(tables());
		await ensureProvisioned(db as never, PRINCIPAL, FACTS);
		const renamed: ForgeFacts = {
			...FACTS,
			orgs: [{ id: '11', login: 'sensei-hq-renamed', role: 'admin' }]
		};
		await ensureProvisioned(db as never, PRINCIPAL, renamed);
		expect(db.tables.tenants.rows.filter((t) => t.origin === 'organization')).toHaveLength(2);
		expect(db.tables.tenant_connections.rows.filter((c) => c.external_id === '11')).toHaveLength(1);
	});

	it('does NOT overwrite a role an admin has overridden', async () => {
		// memberships.role is "usually git-derived, admin-overridable". Re-deriving
		// it on every sign-in would silently undo every override, and Part I
		// Scenario 5 says existing memberships are unchanged.
		const db = fakeDojoDb(tables());
		await ensureProvisioned(db as never, PRINCIPAL, FACTS);
		const acme = db.tables.tenants.rows.find((t) => t.key === 'organization/acme');
		const m = db.tables.memberships.rows.find((r) => r.tenant_id === acme?.id);
		m!.role = 'maintainer'; // an admin promoted them

		await ensureProvisioned(db as never, PRINCIPAL, FACTS);
		expect(
			db.tables.memberships.rows.find((r) => r.tenant_id === acme?.id)?.role
		).toBe('maintainer');
	});

	it('never removes a membership for an org the forge no longer reports', async () => {
		// De-provisioning is phase 2 (§IV.6), and doing it here would be
		// dangerous: only a pass that POSITIVELY proved the forge list may ever
		// remove, or a GitHub outage disables an entire org.
		const db = fakeDojoDb(tables());
		await ensureProvisioned(db as never, PRINCIPAL, FACTS);
		await ensureProvisioned(db as never, PRINCIPAL, { ...FACTS, orgs: [] });
		expect(db.tables.memberships.rows.filter((m) => m.kind === 'employer')).toHaveLength(2);
	});
});

describe('ensureProvisioned — the identity', () => {
	it('records the forge identity against the caller principal', async () => {
		const db = fakeDojoDb(tables());
		await ensureProvisioned(db as never, PRINCIPAL, FACTS);
		expect(db.tables.identities.rows).toHaveLength(1);
		expect(db.tables.identities.rows[0]).toMatchObject({
			principal_id: PRINCIPAL,
			provider: 'github_oauth', // dojo.auth_method — no generic `oauth` yet (§VIII.6)
			subject: '4242' // the forge user id, not the login
		});
	});

	it('refuses to re-point a forge account that already belongs to someone else', async () => {
		// One GitHub account is one person. Upserting on (provider, subject)
		// would silently steal the proof and hand a second principal every
		// membership derived from it.
		const db = fakeDojoDb(tables());
		await ensureProvisioned(db as never, PRINCIPAL, FACTS);
		const err = await ensureProvisioned(db as never, 'p-impostor', FACTS).catch((e) => e);
		expect(err.status).toBe(409);
		expect(db.tables.identities.rows).toHaveLength(1);
		expect(db.tables.identities.rows[0].principal_id).toBe(PRINCIPAL);
	});

	it('is idempotent for the same principal', async () => {
		const db = fakeDojoDb(tables());
		await ensureProvisioned(db as never, PRINCIPAL, FACTS);
		await ensureProvisioned(db as never, PRINCIPAL, FACTS);
		expect(db.tables.identities.rows).toHaveLength(1);
	});
});

describe('ensureProvisioned — without forge facts', () => {
	it('reports synced:false with a reason instead of appearing to succeed', async () => {
		// The failure mode that hid the original bug for two days: the endpoint
		// returned 200 and did nothing. A refusal has to name itself.
		const db = fakeDojoDb(tables());
		const out = await ensureProvisioned(db as never, PRINCIPAL, null, {
			email: 'magic@example.com'
		});
		expect(out.synced).toBe(false);
		expect(out.reason).toBe('no_forge_token');
		expect(db.tables.tenant_connections.rows).toHaveLength(0);
	});

	it('still gives a token-less user their personal dōjō, slugged from their email', async () => {
		// D1 is unconditional: every authenticated user has a personal dōjō. A
		// magic-link user has no forge, so the slug comes from the email local
		// part — a derived name, not an invented identity.
		const db = fakeDojoDb(tables());
		const out = await ensureProvisioned(db as never, PRINCIPAL, null, {
			email: 'magic@example.com'
		});
		expect(out.personal?.key).toBe('personal/magic');
		expect(out.tenants).toEqual([]);
	});

	it('creates nothing when there is no forge login and no email to name it from', async () => {
		// Rather than minting `user-abc123`. A tenant is a governance boundary
		// and its URL is user-visible; a synthesised one is a fabricated identity.
		const db = fakeDojoDb(tables());
		const out = await ensureProvisioned(db as never, PRINCIPAL, null, {});
		expect(out.personal).toBeNull();
		expect(out.reason).toBe('no_identity');
		expect(db.tables.tenants.rows).toHaveLength(0);
	});
});

/** A fetch stub for `GET /repos/{owner}/{repo}`, answering by `owner/repo`. An
 *  unlisted repository answers 404, which is what a token that cannot see it
 *  really gets. */
function repoFetch(answers: Record<string, { status?: number; private?: boolean }>) {
	const calls: string[] = [];
	const fn = (async (url: string | URL | Request) => {
		const u = String(url);
		calls.push(u);
		const key = Object.keys(answers).find((k) => u.endsWith(`/repos/${k}`));
		const a = key ? answers[key] : undefined;
		const status = a?.status ?? (a ? 200 : 404);
		return {
			ok: status >= 200 && status < 300,
			status,
			json: async () => ({ private: a?.private })
		} as Response;
	}) as unknown as typeof fetch;
	return { fn, calls };
}

const MINE = { id: 'm-mine', tenant_id: 't-mine', user_id: PRINCIPAL, role: 'admin' };

/** One `dojo.repositories` row, uncaptured unless said otherwise. */
function repoRow(over: Record<string, unknown> = {}) {
	return {
		id: 'r1',
		tenant_id: 't-mine',
		repo_key: 'github.com/sensei-hq/dbd',
		provider: 'github',
		visibility: null,
		visibility_captured_at: null,
		...over
	};
}

describe('refreshForgeVisibility — capturing the forge answer at sign-in', () => {
	it('does not touch a tenant whose membership has been DISABLED', async () => {
		// Two independent review lenses found this. `dojo.all_my_repositories` and
		// `can_read_repository_metric` both carry `disabled_at is null`; this scope did
		// not — so a member whose access was REVOKED still had their forge token write
		// visibility on that tenant's repositories, and visibility drives WHICH
		// AUTHORITY governs sharing for every remaining member of it.
		const revoked = { ...MINE, disabled_at: '2026-01-01T00:00:00.000Z' };
		const db = fakeDojoDb(
			tables({ memberships: { rows: [revoked] }, repositories: { rows: [repoRow()] } })
		);
		const { fn, calls } = repoFetch({ 'sensei-hq/dbd': { private: false } });
		const out = await refreshForgeVisibility(db as never, PRINCIPAL, 'tok', fn);

		expect(calls).toHaveLength(0);
		expect(out.captured).toBe(0);
		expect(db.tables.repositories.rows[0].visibility).toBeNull();
	});

	it('writes visibility AND visibility_captured_at onto a row that already exists', async () => {
		// `dojo.repositories.visibility` may come from exactly one place: the forge.
		const db = fakeDojoDb(
			tables({ memberships: { rows: [MINE] }, repositories: { rows: [repoRow()] } })
		);
		const { fn, calls } = repoFetch({ 'sensei-hq/dbd': { private: false } });
		const out = await refreshForgeVisibility(db as never, PRINCIPAL, 'tok', fn);

		expect(out).toEqual({
			captured: 1,
			unavailable: 0,
			failed: 0,
			deferred: 0,
			unsupported: 0
		});
		expect(calls).toEqual(['https://api.github.com/repos/sensei-hq/dbd']);
		const row = db.tables.repositories.rows[0];
		expect(row.visibility).toBe('public');
		// A value with no timestamp is indistinguishable from the old bad default,
		// and the view's staleness rule has nothing to measure. Always both.
		expect(Date.parse(String(row.visibility_captured_at))).not.toBeNaN();
	});

	it('re-stamps captured_at even when the value did not change', async () => {
		// Freshness IS the guard: §8a shows a stale `public` keeps a now-private
		// repository syncing free under an election nobody re-made. So a pass that
		// confirms the same value must still move the clock.
		const db = fakeDojoDb(
			tables({
				memberships: { rows: [MINE] },
				repositories: {
					rows: [
						repoRow({ visibility: 'private', visibility_captured_at: '2020-01-01T00:00:00.000Z' })
					]
				}
			})
		);
		const { fn } = repoFetch({ 'sensei-hq/dbd': { private: true } });
		await refreshForgeVisibility(db as never, PRINCIPAL, 'tok', fn);

		const row = db.tables.repositories.rows[0];
		expect(row.visibility).toBe('private');
		expect(Date.parse(String(row.visibility_captured_at))).toBeGreaterThan(
			Date.parse('2020-01-01T00:00:00.000Z')
		);
	});

	it('NEVER inserts a repository row — it refreshes, it does not inventory', async () => {
		// The rejected design: a row per visible forge repo. That discloses repos
		// the user never chose to disclose, turning a sign-in into an inventory
		// upload (§8a item 2, REJECTED). So with nothing registered there is
		// nothing to ask the forge about, and nothing to write.
		const db = fakeDojoDb(tables({ memberships: { rows: [MINE] } }));
		const { fn, calls } = repoFetch({ 'sensei-hq/dbd': { private: false } });
		const out = await refreshForgeVisibility(db as never, PRINCIPAL, 'tok', fn);

		expect(db.tables.repositories.rows).toHaveLength(0);
		expect(calls).toEqual([]);
		expect(out.captured).toBe(0);
	});

	it('leaves a row in a tenant the signer does not belong to untouched', async () => {
		// THE AUTHORIZATION BOUNDARY. `unique (tenant_id, repo_key)` means the same
		// repository exists under two tenants, so an unscoped update by repo_key
		// would use user A's token to rewrite tenant B's row — and since visibility
		// decides authority, that changes who governs it for every member of B.
		const db = fakeDojoDb(
			tables({
				memberships: { rows: [MINE] },
				repositories: {
					rows: [
						repoRow({ id: 'r-mine', repo_key: 'github.com/acme/app' }),
						repoRow({
							id: 'r-theirs',
							tenant_id: 't-theirs',
							repo_key: 'github.com/acme/app',
							visibility: 'private',
							visibility_captured_at: '2020-01-01T00:00:00.000Z'
						})
					]
				}
			})
		);
		const { fn } = repoFetch({ 'acme/app': { private: false } });
		const out = await refreshForgeVisibility(db as never, PRINCIPAL, 'tok', fn);

		expect(out.captured).toBe(1);
		const mine = db.tables.repositories.rows.find((r) => r.id === 'r-mine');
		const theirs = db.tables.repositories.rows.find((r) => r.id === 'r-theirs');
		expect(mine?.visibility).toBe('public');
		// Not merely "not public" — byte-for-byte what it was.
		expect(theirs?.visibility).toBe('private');
		expect(theirs?.visibility_captured_at).toBe('2020-01-01T00:00:00.000Z');
	});

	it('leaves the row uncaptured when the forge read fails', async () => {
		// Never a guessed `private`. In an org tenant that guess resolves to
		// ORG-MANDATED and shares the repository with no election by anyone.
		const db = fakeDojoDb(
			tables({ memberships: { rows: [MINE] }, repositories: { rows: [repoRow()] } })
		);
		const { fn } = repoFetch({ 'sensei-hq/dbd': { status: 503 } });
		const out = await refreshForgeVisibility(db as never, PRINCIPAL, 'tok', fn);

		expect(out).toMatchObject({ captured: 0, failed: 1 });
		const row = db.tables.repositories.rows[0];
		expect(row.visibility).toBeNull();
		expect(row.visibility_captured_at).toBeNull();
	});

	it('leaves the row alone on 404 rather than writing a value', async () => {
		// No access, or renamed upstream. Reported apart from a fault because they
		// are different problems: one needs a scope, the other needs a retry.
		const db = fakeDojoDb(
			tables({
				memberships: { rows: [MINE] },
				repositories: { rows: [repoRow({ repo_key: 'github.com/acme/gone' })] }
			})
		);
		const { fn } = repoFetch({});
		const out = await refreshForgeVisibility(db as never, PRINCIPAL, 'tok', fn);

		expect(out).toMatchObject({ captured: 0, unavailable: 1, failed: 0 });
		expect(db.tables.repositories.rows[0].visibility).toBeNull();
		expect(db.tables.repositories.rows[0].visibility_captured_at).toBeNull();
	});

	it('reads the forge once for a repository registered under two of the signer tenants', async () => {
		const db = fakeDojoDb(
			tables({
				memberships: {
					rows: [MINE, { id: 'm-org', tenant_id: 't-org', user_id: PRINCIPAL, role: 'contributor' }]
				},
				repositories: {
					rows: [
						repoRow({ id: 'r-a', repo_key: 'github.com/acme/app' }),
						repoRow({ id: 'r-b', tenant_id: 't-org', repo_key: 'github.com/acme/app' })
					]
				}
			})
		);
		const { fn, calls } = repoFetch({ 'acme/app': { private: true } });
		const out = await refreshForgeVisibility(db as never, PRINCIPAL, 'tok', fn);

		expect(calls).toHaveLength(1);
		expect(out.captured).toBe(2);
		expect(db.tables.repositories.rows.map((r) => r.visibility)).toEqual(['private', 'private']);
	});

	it('does not address a repository on a forge this token cannot speak for', async () => {
		// A GitHub token proves nothing about a GitLab repository, and an
		// unattributable key has no owner/repo to ask about. Both leave the row
		// uncaptured, which fails closed.
		const db = fakeDojoDb(
			tables({
				memberships: { rows: [MINE] },
				repositories: {
					rows: [
						repoRow({ id: 'r-gl', repo_key: 'gitlab.com/acme/app', provider: 'gitlab' }),
						repoRow({ id: 'r-odd', repo_key: 'not-a-forge-key' })
					]
				}
			})
		);
		const { fn, calls } = repoFetch({ 'acme/app': { private: false } });
		const out = await refreshForgeVisibility(db as never, PRINCIPAL, 'tok', fn);

		expect(calls).toEqual([]);
		expect(out).toMatchObject({ captured: 0, unsupported: 2 });
		expect(db.tables.repositories.rows.every((r) => r.visibility === null)).toBe(true);
	});

	it('defers the overflow past the per-pass cap, oldest capture first', async () => {
		// A Worker has a hard per-invocation subrequest budget — the metrics ingest
		// already lost a whole batch to it. So the pass is BOUNDED, and what it
		// leaves behind is reported rather than silently dropped. Uncaptured rows
		// go first: they are the ones that cannot sync at all.
		const cap = MAX_VISIBILITY_REFRESH_PER_PASS;
		// Seeded in the WRONG order deliberately: the uncaptured rows come LAST, and
		// the captured ones ascend in age, so an implementation that simply took the
		// first `cap` rows would defer exactly the three that cannot sync at all.
		const rows = Array.from({ length: cap + 3 }, (_, i) =>
			repoRow({
				id: `r-${i}`,
				repo_key: `github.com/acme/repo-${i}`,
				visibility: i < cap ? 'private' : null,
				visibility_captured_at: i < cap ? new Date(2020, 0, 1 + i).toISOString() : null
			})
		);
		const answers = Object.fromEntries(
			rows.map((_, i) => [`acme/repo-${i}`, { private: false }])
		);
		const db = fakeDojoDb(
			tables({ memberships: { rows: [MINE] }, repositories: { rows } })
		);
		const { fn, calls } = repoFetch(answers);
		const out = await refreshForgeVisibility(db as never, PRINCIPAL, 'tok', fn);

		expect(calls).toHaveLength(cap);
		expect(out).toMatchObject({ captured: cap, deferred: 3 });
		// The uncaptured ones were served first, despite being seeded last…
		for (const i of [cap, cap + 1, cap + 2]) {
			expect(db.tables.repositories.rows.find((r) => r.id === `r-${i}`)?.visibility).toBe('public');
		}
		// …and the three most recently captured were left for the next pass.
		for (const i of [cap - 3, cap - 2, cap - 1]) {
			const row = db.tables.repositories.rows.find((r) => r.id === `r-${i}`);
			expect(row?.visibility).toBe('private');
			expect(row?.visibility_captured_at).toBe(new Date(2020, 0, 1 + i).toISOString());
		}
	});

	it('does nothing when the signer belongs to no tenant', async () => {
		// No membership, no scope, no rows — and no forge traffic to prove it.
		const db = fakeDojoDb(tables({ repositories: { rows: [repoRow()] } }));
		const { fn, calls } = repoFetch({ 'sensei-hq/dbd': { private: false } });
		const out = await refreshForgeVisibility(db as never, PRINCIPAL, 'tok', fn);

		expect(calls).toEqual([]);
		expect(out.captured).toBe(0);
		expect(db.tables.repositories.rows[0].visibility).toBeNull();
	});
});

describe('provisionWithToken — the composition all three callers share', () => {
	/** A fetch stub standing in for the GitHub API. */
	function forgeFetch(status: number, user: unknown, orgs: unknown) {
		return (async (url: string | URL | Request) => {
			const u = String(url);
			const body = u.includes('/memberships/orgs') ? orgs : user;
			return { ok: status >= 200 && status < 300, status, json: async () => body } as Response;
		}) as unknown as typeof fetch;
	}

	const GH_USER = { id: 4242, login: 'jerrythomas', name: 'Jerry Thomas', email: 'j@example.com' };
	const GH_ORGS = [{ state: 'active', role: 'admin', organization: { id: 11, login: 'sensei-hq' } }];

	it('reads the forge and provisions everything when a token is present', async () => {
		const db = fakeDojoDb(tables());
		const out = await provisionWithToken(
			db as never,
			PRINCIPAL,
			'gh-token',
			{},
			forgeFetch(200, GH_USER, GH_ORGS)
		);
		expect(out.synced).toBe(true);
		expect(out.personal?.key).toBe('personal/jerrythomas');
		expect(out.tenants.map((t) => t.key)).toEqual(['organization/sensei-hq']);
	});

	it('reports forge_unreachable — distinctly from no_forge_token — when the API fails', async () => {
		// These are different problems and the console says different things about
		// them. Collapsing both into "nothing to sync" is the shape that hid the
		// original bug for two days.
		const db = fakeDojoDb(tables());
		const out = await provisionWithToken(
			db as never,
			PRINCIPAL,
			'gh-token',
			{ email: 'j@example.com' },
			forgeFetch(503, GH_USER, GH_ORGS)
		);
		expect(out.synced).toBe(false);
		expect(out.reason).toBe('forge_unreachable');
		// No org tenant is invented from a failed read…
		expect(db.tables.tenant_connections.rows).toHaveLength(0);
		// …but D1 still holds: the personal dōjō does not depend on the forge.
		expect(out.personal?.key).toBe('personal/j');
	});

	it('reports forge_token_rejected — not forge_unreachable — when GitHub says 401', async () => {
		// A REVOKED grant and a DOWN forge want opposite advice. Both used to
		// produce 'forge_unreachable', whose console copy is "try again in a
		// moment" — advice that can never come true for a dead token, and the
		// daemon retried it every 60s forever on the strength of it.
		//
		// 401 is the unambiguous one: GitHub means "these credentials are bad".
		const db = fakeDojoDb(tables());
		const out = await provisionWithToken(
			db as never,
			PRINCIPAL,
			'revoked-token',
			{ email: 'j@example.com' },
			forgeFetch(401, GH_USER, GH_ORGS)
		);
		expect(out.synced).toBe(false);
		expect(out.reason).toBe('forge_token_rejected');
		// Same safety property as the unreachable case: nothing is invented from
		// a read that did not succeed.
		expect(db.tables.tenant_connections.rows).toHaveLength(0);
		expect(out.personal?.key).toBe('personal/j');
	});

	it('keeps a 403 as forge_unreachable, because it is not always the token', async () => {
		// GitHub answers 403 for a rate limit as well as for SSO/scope refusals.
		// Telling a rate-limited user to sign in again would be a wrong remedy, so
		// only 401 is treated as a dead credential.
		const db = fakeDojoDb(tables());
		const out = await provisionWithToken(
			db as never,
			PRINCIPAL,
			'gh-token',
			{ email: 'j@example.com' },
			forgeFetch(403, GH_USER, GH_ORGS)
		);
		expect(out.reason).toBe('forge_unreachable');
	});

	it('reports no_forge_token when there is no token at all', async () => {
		const db = fakeDojoDb(tables());
		const out = await provisionWithToken(db as never, PRINCIPAL, null, {
			email: 'magic@example.com'
		});
		expect(out.synced).toBe(false);
		expect(out.reason).toBe('no_forge_token');
	});

	/** The forge, answering all three reads one pass makes. */
	function fullForgeFetch(repos: Record<string, boolean>) {
		const calls: string[] = [];
		return {
			calls,
			fn: (async (url: string | URL | Request) => {
				const u = String(url);
				calls.push(u);
				if (u.includes('/repos/')) {
					const key = Object.keys(repos).find((k) => u.endsWith(`/repos/${k}`));
					if (!key) return { ok: false, status: 404, json: async () => ({}) } as Response;
					return { ok: true, status: 200, json: async () => ({ private: repos[key] }) } as Response;
				}
				const body = u.includes('/memberships/orgs') ? GH_ORGS : GH_USER;
				return { ok: true, status: 200, json: async () => body } as Response;
			}) as unknown as typeof fetch
		};
	}

	/** A caller who already has a personal tenant with one registered repository —
	 *  the state `registerRepositories` leaves behind, with visibility uncaptured. */
	function withRegisteredRepo() {
		return tables({
			tenants: {
				rows: [
					{
						id: 't-mine',
						key: 'personal/jerrythomas',
						origin: 'personal',
						slug: 'jerrythomas',
						name: 'Jerry'
					}
				],
				uniques: [{ columns: ['key'] }]
			},
			memberships: {
				rows: [{ id: 'm1', tenant_id: 't-mine', user_id: PRINCIPAL, role: 'admin', kind: 'personal' }],
				uniques: [{ columns: ['tenant_id', 'user_id'] }]
			},
			repositories: {
				rows: [
					{
						id: 'r1',
						tenant_id: 't-mine',
						repo_key: 'github.com/jerrythomas/thing',
						provider: 'github',
						visibility: null,
						visibility_captured_at: null
					}
				]
			}
		});
	}

	it('captures forge visibility at sign-in — the one moment a forge token exists', async () => {
		// The chicken/egg §8a resolves: registration holds a SUPABASE token and
		// cannot ask the forge, so capture rides the provisioning pass, which is the
		// only server-side place `provider_token` is reachable.
		const db = fakeDojoDb(withRegisteredRepo());
		const forge = fullForgeFetch({ 'jerrythomas/thing': false });
		const out = await provisionWithToken(db as never, PRINCIPAL, 'gh-token', {}, forge.fn);

		expect(out.synced).toBe(true);
		expect(out.visibility).toMatchObject({ captured: 1 });
		const row = db.tables.repositories.rows[0];
		expect(row.visibility).toBe('public');
		expect(Date.parse(String(row.visibility_captured_at))).not.toBeNaN();
	});

	it('does not touch the forge for visibility when there is no token', async () => {
		const db = fakeDojoDb(withRegisteredRepo());
		const out = await provisionWithToken(db as never, PRINCIPAL, null, {
			email: 'j@example.com'
		});
		expect(out.visibility).toBeUndefined();
		expect(db.tables.repositories.rows[0].visibility).toBeNull();
	});

	it('does not capture visibility from a failed forge read', async () => {
		// `forge_unreachable` provisions no org tenant; it must equally capture no
		// visibility. A pass that could not read the forge learned nothing.
		const db = fakeDojoDb(withRegisteredRepo());
		const out = await provisionWithToken(
			db as never,
			PRINCIPAL,
			'gh-token',
			{ email: 'j@example.com' },
			forgeFetch(503, GH_USER, GH_ORGS)
		);
		expect(out.reason).toBe('forge_unreachable');
		expect(out.visibility).toBeUndefined();
		expect(db.tables.repositories.rows[0].visibility).toBeNull();
	});
});

// ── Concurrent provisioning must converge, not fork ─────────────────────────
//
// OBSERVED LIVE, not hypothesised. Signing in produced NINE tenants where five
// were expected: `senecaglobalinc` AND `senecaglobalinc-2`, `jovy-thomas-visuals`
// AND `-2`, `jerrythomas` AND `-2`. The duplicates were created 2 MILLISECONDS
// apart — two provisioning passes in flight at once (kavach's `onSessionSync`
// and the console's `POST /v1/you/provision`), both missing the lookup, both
// inserting.
//
// The duplicates were diagnosable by a property: every `-2` tenant had NO
// `tenant_connections` row and exactly one membership. That is the signature of
// the loser — it created a tenant, then its OWN connection insert hit 23505,
// which the code swallowed, leaving an orphan tenant the user could see and a
// membership pointing at it.
//
// `23505` on the connection is not noise. It is the DEFINITIVE signal that this
// forge org already belongs to a tenant — i.e. "you lost, adopt the winner".
import { adoptConnectedTenant } from './provisioning';

describe('adoptConnectedTenant — losing the race must JOIN, never fork', () => {
	function raceTables(): Record<string, FakeTable> {
		return {
			tenants: {
				rows: [
					{ id: 't-winner', key: 'organization/acme', origin: 'organization', slug: 'acme' },
					// what THIS pass created a moment ago, before discovering it lost
					{ id: 't-mine', key: 'organization/acme-2', origin: 'organization', slug: 'acme-2' }
				],
				uniques: [{ columns: ['key'] }]
			},
			tenant_connections: {
				rows: [{ id: 'c1', tenant_id: 't-winner', provider: 'github', external_id: '44767229' }],
				uniques: [{ columns: ['provider', 'external_id'] }]
			}
		};
	}

	it('returns the tenant the forge org is ALREADY connected to', async () => {
		const db = fakeDojoDb(raceTables());
		const won = await adoptConnectedTenant(db as never, 'github', '44767229', 't-mine');
		expect(won).toMatchObject({ id: 't-winner', key: 'organization/acme' });
	});

	it('removes the redundant tenant it just created, so no `-2` survives', async () => {
		// This is the whole user-visible bug: without the delete, the orphan shows
		// up in "My dōjōs" as a second copy of an organisation you belong to once.
		const db = fakeDojoDb(raceTables());
		await adoptConnectedTenant(db as never, 'github', '44767229', 't-mine');
		const keys = db.tables.tenants.rows.map((r) => r.key);
		expect(keys).toEqual(['organization/acme']);
	});

	it('never deletes the winner, even when asked to discard it', async () => {
		// Defensive: if the tenant we "created" IS the connected one, we did not
		// actually lose — deleting it would destroy the real tenant and its
		// memberships. Cheap guard against a caller passing the wrong id.
		const db = fakeDojoDb(raceTables());
		await adoptConnectedTenant(db as never, 'github', '44767229', 't-winner');
		expect(db.tables.tenants.rows.map((r) => r.key)).toContain('organization/acme');
	});

	it('THROWS rather than inventing a tenant when no connection exists', async () => {
		// Only reachable if 23505 fired without a conflicting row — i.e. our model
		// of the constraint is wrong. Say so; returning the tenant we created would
		// re-introduce the fork this function exists to prevent.
		const db = fakeDojoDb(raceTables());
		await expect(
			adoptConnectedTenant(db as never, 'github', 'no-such-org', 't-mine')
		).rejects.toThrow();
	});
});

// The PERSONAL half of the same race: `personal/jerrythomas` AND
// `personal/jerrythomas-2`, also 2ms apart.
//
// The `-2` escalation is CORRECT for two different humans who are both
// `jerrythomas` on two forges — that is exactly why it exists (§II.3). It is
// wrong when a pass collides with ITSELF. The distinguisher is membership: if
// the colliding tenant already has a membership for this same principal, it is
// mine and I raced myself; if it does not, it is someone else's and I must land
// somewhere of my own.
import { adoptOwnPersonalTenant } from './provisioning';

describe('adoptOwnPersonalTenant — tell "I raced myself" from "another human, same name"', () => {
	const ALICE = 'p-alice';
	function tbl(memberUserId: string | null): Record<string, FakeTable> {
		return {
			tenants: {
				rows: [{ id: 't-existing', key: 'personal/jerrythomas', origin: 'personal', slug: 'jerrythomas' }],
				uniques: [{ columns: ['key'] }]
			},
			memberships: {
				rows: memberUserId ? [{ id: 'm1', tenant_id: 't-existing', user_id: memberUserId }] : [],
				uniques: [{ columns: ['tenant_id', 'user_id'] }]
			}
		};
	}

	it('adopts the colliding tenant when THIS principal is already a member', async () => {
		const db = fakeDojoDb(tbl(ALICE));
		await expect(
			adoptOwnPersonalTenant(db as never, 'personal/jerrythomas', ALICE)
		).resolves.toMatchObject({ id: 't-existing', key: 'personal/jerrythomas' });
	});

	it('returns null for a DIFFERENT human with the same login, so -2 still happens', async () => {
		// Losing this distinction would silently join two unrelated people into one
		// dōjō — far worse than a duplicate.
		const db = fakeDojoDb(tbl('p-someone-else'));
		await expect(adoptOwnPersonalTenant(db as never, 'personal/jerrythomas', ALICE)).resolves.toBeNull();
	});

	it('returns null when the colliding tenant has no members at all', async () => {
		const db = fakeDojoDb(tbl(null));
		await expect(adoptOwnPersonalTenant(db as never, 'personal/jerrythomas', ALICE)).resolves.toBeNull();
	});
});

// ── The CLAIM (§II.4) ───────────────────────────────────────────────────────
//
// An org tenant is created by whoever signs in FIRST, who may be a plain member
// — so its existence proves nothing about who owns the org. An unclaimed tenant
// may not hold a subscription, and therefore can never sync private data.
//
// The claim is not new information: `roleForOrg` already reads `org.role` at
// every sign-in. So a forge owner/admin signing in IS the proof, and recording
// it needs no new endpoint and no prompt.
//
// SHIPPED WITH THE GATE, DELIBERATELY. Adding `unclaimed` to the view without
// this would repeat spec finding F3 verbatim — a gate nothing can satisfy, which
// refuses every org repository forever and reads as "nothing to sync".
describe('claimTenantIfOwner — a forge owner signing in IS the claim', () => {
	const ALICE = 'p-alice';
	function tbl(over: Record<string, unknown> = {}): Record<string, FakeTable> {
		return {
			tenants: {
				rows: [
					{
						id: 't-acme',
						key: 'organization/acme',
						origin: 'organization',
						slug: 'acme',
						claimed_at: null,
						claimed_by: null,
						...over
					}
				],
				uniques: [{ columns: ['key'] }]
			}
		};
	}

	it('claims an unclaimed tenant for a forge ADMIN', async () => {
		const db = fakeDojoDb(tbl());
		const { claimTenantIfOwner } = await import('./provisioning');
		await claimTenantIfOwner(db as never, 't-acme', ALICE, 'admin');
		const row = db.tables.tenants.rows[0];
		expect(row.claimed_by).toBe(ALICE);
		expect(row.claimed_at).toBeTruthy();
	});

	it('does NOT claim for a contributor — membership is not ownership', async () => {
		// The whole point of the claim: being IN an org says nothing about owning
		// it, and a tenant claimed by a plain member could subscribe on behalf of
		// an organisation that never agreed.
		const db = fakeDojoDb(tbl());
		const { claimTenantIfOwner } = await import('./provisioning');
		await claimTenantIfOwner(db as never, 't-acme', ALICE, 'contributor');
		expect(db.tables.tenants.rows[0].claimed_at).toBeNull();
	});

	it('leaves an ALREADY-claimed tenant alone, including who claimed it', async () => {
		// Re-stamping on every sign-in would rewrite `claimed_by` to whichever
		// admin logged in most recently, destroying the record of who actually
		// established the claim.
		const db = fakeDojoDb(
			tbl({ claimed_at: '2026-01-01T00:00:00.000Z', claimed_by: 'p-first-owner' })
		);
		const { claimTenantIfOwner } = await import('./provisioning');
		await claimTenantIfOwner(db as never, 't-acme', ALICE, 'admin');
		const row = db.tables.tenants.rows[0];
		expect(row.claimed_by).toBe('p-first-owner');
		expect(row.claimed_at).toBe('2026-01-01T00:00:00.000Z');
	});

	it('reports a write failure rather than swallowing it', async () => {
		// A silently-failed claim leaves the tenant unclaimed, which now REFUSES
		// every private repository — a denial caused by an error nobody saw.
		const failing = {
			from: () => failing,
			select: () => failing,
			eq: () => failing,
			maybeSingle: async () => ({ data: { id: 't-acme', claimed_at: null }, error: null }),
			update: () => ({
				eq: () => ({ is: async () => ({ error: { message: 'write failed' } }) })
			})
		} as never;
		const { claimTenantIfOwner } = await import('./provisioning');
		await expect(claimTenantIfOwner(failing, 't-acme', ALICE, 'admin')).rejects.toThrow();
	});
});
