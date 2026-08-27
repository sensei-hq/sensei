// ensureProvisioned — the operation that closes the hole this whole slice is
// about: nothing in the dōjō created a tenant. Spec §II.7, §VIII.7 item 2.
//
// These run against `fakeDojoDb`, which actually stores rows and enforces the
// real unique constraints, because the headline property is IDEMPOTENCE — and a
// queue stub that replays canned answers would pass just as happily for an
// implementation that inserted a duplicate tenant on every sign-in.
import { describe, it, expect, beforeEach } from 'vitest';
import { fakeDojoDb, resetFakeIds, type FakeTable } from './fake-dojo-db';
import { ensureProvisioned } from './provisioning';
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
