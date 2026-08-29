// Regression tests for `dojo-orgs.ts` — the DojoOrg view-model builder consumed
// by the org picker and the console shell load. Focus: FAIL CLOSED on a DB error
// (throw 500, never a fabricated empty list) so a real member is never silently
// ejected to the solo/personal landing on a transient failure; a GENUINE miss
// still returns empty/undefined. A chainable supabase-js stub (no live DB).
import { describe, it, expect, vi } from 'vitest';

type Terminal = { data: unknown; error: unknown };

// Chainable stub: `.maybeSingle()` shifts the next queued terminal; awaiting the
// builder (the memberships list query) resolves the next terminal too.
function makeDb(...results: Terminal[]) {
	const queue = [...results];
	const next = () => queue.shift() ?? { data: null, error: null };
	const b: Record<string, unknown> = {};
	b.from = () => b;
	b.select = () => b;
	b.eq = () => b;
	b.is = () => b;
	b.maybeSingle = () => Promise.resolve(next());
	b.then = (resolve: (v: Terminal) => unknown) => resolve(next());
	return b;
}

let stub: unknown;
vi.mock('./dojo-supabase', async (importOriginal) => {
	const actual = await (importOriginal as () => Promise<Record<string, unknown>>)();
	return { ...actual, dojoDb: () => stub };
});

const { listUserOrgs, getUserOrg, tenantToOrg, TENANT_COLS } = await import('./dojo-orgs');
const { orgKindKanji } = await import('../dojo-data');

const TENANT_ROW = {
	id: 't1',
	key: 'organization/acme',
	slug: 'acme',
	name: 'Acme',
	self_hosted: false,
	// The DISPLAYED kind derives from this, not from `memberships.kind`.
	origin: 'organization'
};

describe('listUserOrgs — fail closed on a memberships query error', () => {
	it('throws 500 (never a fabricated empty list) on a DB error', async () => {
		stub = makeDb({ data: null, error: { message: 'boom' } });
		const err = await listUserOrgs('u1').catch((e) => e);
		expect(err?.status).toBe(500);
	});

	it('returns [] when the user genuinely has no memberships (no error)', async () => {
		stub = makeDb({ data: [], error: null });
		expect(await listUserOrgs('u1')).toEqual([]);
	});

	it('maps real membership rows to DojoOrg records', async () => {
		stub = makeDb({ data: [{ role: 'admin', tenant: TENANT_ROW }], error: null });
		const orgs = await listUserOrgs('u1');
		expect(orgs).toHaveLength(1);
		expect(orgs[0]).toMatchObject({ id: 't1', url: 'organization/acme', name: 'Acme', role: 'Admin' });
	});

	it('derives the kind + kanji from the tenant ORIGIN, not the membership tag', async () => {
		stub = makeDb({ data: [{ role: 'admin', kind: 'employer', tenant: TENANT_ROW }], error: null });
		const orgs = await listUserOrgs('u1');
		expect(orgs[0].kind).toBe('Organization');
		expect(orgs[0].kanji).toBe('社');
	});

	it('does NOT fabricate counts — members/projects/pending are undefined when not computed', async () => {
		stub = makeDb({ data: [{ role: 'admin', kind: 'client', tenant: TENANT_ROW }], error: null });
		const [org] = await listUserOrgs('u1');
		// `kind: 'client'` in the row is ignored — origin decides.
		expect(org.kind).toBe('Organization');
		expect(org.members).toBeUndefined();
		expect(org.projects).toBeUndefined();
		expect(org.pending).toBeUndefined();
	});
});

describe('getUserOrg — fail closed on either lookup error', () => {
	it('throws 500 when the tenant lookup errors', async () => {
		stub = makeDb({ data: null, error: { message: 'tenant boom' } });
		const err = await getUserOrg('u1', 'organization/acme').catch((e) => e);
		expect(err?.status).toBe(500);
	});

	it('throws 500 when the membership lookup errors', async () => {
		stub = makeDb({ data: TENANT_ROW, error: null }, { data: null, error: { message: 'mem boom' } });
		const err = await getUserOrg('u1', 'organization/acme').catch((e) => e);
		expect(err?.status).toBe(500);
	});

	it('returns undefined on a genuine tenant miss (no error, no row)', async () => {
		stub = makeDb({ data: null, error: null });
		expect(await getUserOrg('u1', 'gh/none')).toBeUndefined();
	});

	it('returns undefined when the tenant exists but the user is not a member', async () => {
		stub = makeDb({ data: TENANT_ROW, error: null }, { data: null, error: null });
		expect(await getUserOrg('u1', 'organization/acme')).toBeUndefined();
	});

	it('maps to a DojoOrg on a real hit', async () => {
		stub = makeDb({ data: TENANT_ROW, error: null }, { data: { role: 'lead' }, error: null });
		const org = await getUserOrg('u1', 'organization/acme');
		expect(org).toMatchObject({ id: 't1', url: 'organization/acme', role: 'Lead' });
	});

	it('derives the kind from the tenant ORIGIN on a hit, ignoring membership.kind', async () => {
		stub = makeDb(
			{ data: TENANT_ROW, error: null },
			{ data: { role: 'lead', kind: 'community' }, error: null }
		);
		const org = await getUserOrg('u1', 'organization/acme');
		expect(org?.kind).toBe('Organization');
		expect(org?.kanji).toBe('社');
	});

	it('shows a PERSONAL tenant as Personal — origin is what separates them', async () => {
		// Guards the failure the origin column exists to prevent: without it in
		// TENANT_COLS every dōjō, personal ones included, reads as Organization.
		stub = makeDb(
			{ data: { ...TENANT_ROW, key: 'personal/jerry', origin: 'personal' }, error: null },
			{ data: { role: 'admin', kind: 'personal' }, error: null }
		);
		const org = await getUserOrg('u1', 'personal/jerry');
		expect(org?.kind).toBe('Personal');
		expect(org?.kanji).toBe('己');
	});
});

describe('orgKindKanji — identity glyph per kind', () => {
	it('maps each kind to its ladder kanji', () => {
		expect(orgKindKanji('Organization')).toBe('社');
		expect(orgKindKanji('Personal')).toBe('己');
	});
});

// The tenants column rename (`org` → `slug`, commit 37ca9fab) reached the
// writers but not this reader: TENANT_COLS still selected `org`, PostgREST
// rejected the query, and `listUserOrgs` correctly failed closed with a 500 —
// which made /you unreachable for every signed-in user. Nothing caught it
// because the spec mocked the client and asserted the payload the code sends.
describe('tenant column rename (org → slug)', () => {
	it('falls back to the tenant SLUG when a tenant has no display name', () => {
		// The only place the column is read rather than just selected. A row
		// carrying `slug` and no `name` must render the slug, not `undefined`.
		const row = { id: 't1', key: 'organization/acme', slug: 'acme', name: null, self_hosted: false };
		expect(tenantToOrg(row as never, 'admin').name).toBe('acme');
	});

	it('does not select a column the database no longer has', () => {
		// TENANT_COLS is the string handed to PostgREST. `org` in it is a hard
		// query error, not a missing field.
		expect(TENANT_COLS).not.toMatch(/\borg\b/);
		expect(TENANT_COLS).toMatch(/\bslug\b/);
	});
});

// ── The auth-id / principal-id confusion ────────────────────────────────────
//
// `dojo.memberships.user_id` holds a PRINCIPAL id. `locals.session.user.id` is
// the SUPABASE AUTH id. `dojo.principals.id` defaults to `gen_random_uuid()`, so
// the two are NEVER equal — not "usually equal with an exception", always
// different. Passing the session id straight into `listUserOrgs` therefore
// matches zero rows for EVERY user.
//
// Verified on live data before this test was written: the same query returns
// 0 rows for the auth id and 2 for the principal id, on an account with two
// active memberships. The symptom is an empty "My dōjōs" — and worse,
// `hasMembership` comes out false, so a real member is treated as solo.
//
// The API plane never had this bug: `resolveCaller` translates via
// `resolvePrincipalId`. This is the PAGE plane catching up.
describe('principalIdForSession — the page plane must translate, like resolveCaller does', () => {
	it('returns the PRINCIPAL id, not the session id it was given', async () => {
		stub = makeDb({ data: { id: 'p-principal' }, error: null });
		const { principalIdForSession } = await import('./dojo-orgs');
		await expect(principalIdForSession({ session: { user: { id: 'auth-uuid' } } })).resolves.toBe(
			'p-principal'
		);
	});

	it('is null when there is no session at all', async () => {
		stub = makeDb();
		const { principalIdForSession } = await import('./dojo-orgs');
		await expect(principalIdForSession({})).resolves.toBeNull();
	});

	it('is null for a signed-in user who has no principal row yet', async () => {
		// A genuine miss, not a failure: a brand-new account has no principal and
		// therefore no memberships. Empty is the truth here.
		stub = makeDb({ data: null, error: null });
		const { principalIdForSession } = await import('./dojo-orgs');
		await expect(principalIdForSession({ session: { user: { id: 'auth-uuid' } } })).resolves.toBeNull();
	});

	it('THROWS on a lookup failure rather than reporting "no principal"', async () => {
		// The same fail-closed rule `listUserOrgs` already follows. Returning null
		// here would degrade to an empty membership list, which silently ejects a
		// real member to the solo landing — indistinguishable from having no orgs.
		stub = makeDb({ data: null, error: { message: 'connection reset' } });
		const { principalIdForSession } = await import('./dojo-orgs');
		await expect(
			principalIdForSession({ session: { user: { id: 'auth-uuid' } } })
		).rejects.toThrow();
	});
});

// ── The employer tag is a claim we cannot substantiate ──────────────────────
//
// Provisioning wrote `kind = 'employer'` for EVERY org it discovered, because
// GitHub cannot tell us which of your organisations employs you. Live data:
// jovy-thomas-visuals, senecaglobalinc and sensei-hq were all "Employer" — one
// is an employer, one is a personal venture, one is a product org.
//
// `origin` IS knowable: the forge says whether a tenant is an org or a personal
// account. So the display derives from origin, and `memberships.kind` is no
// longer read for it. The column stays (NOT NULL, enum-constrained) until there
// is a real way for a human to designate employer vs client — at which point it
// starts meaning something, instead of being asserted on their behalf.
describe('tenantToOrg — the kind shown is the ORIGIN, never the unsubstantiated tag', () => {
	const base = { id: 't1', key: 'organization/acme', slug: 'acme', name: 'Acme', self_hosted: false };

	it('shows an organisation as Organization even though kind says employer', async () => {
		const { tenantToOrg } = await import('./dojo-orgs');
		const o = tenantToOrg({ ...base, origin: 'organization' } as never, 'admin', 'employer');
		expect(o.kind).toBe('Organization');
		expect(o.kanji).toBe('社');
	});

	it('shows a personal tenant as Personal', async () => {
		const { tenantToOrg } = await import('./dojo-orgs');
		const o = tenantToOrg(
			{ ...base, key: 'personal/jerry', origin: 'personal' } as never,
			'admin',
			'personal'
		);
		expect(o.kind).toBe('Personal');
		expect(o.kanji).toBe('己');
	});

	it('IGNORES the stored kind entirely — client/community do not resurface', async () => {
		// The tag is not merely renamed; it is no longer consulted. Were it still
		// read, a `client` row would render "Client" and reintroduce a relationship
		// claim nobody asserted.
		const { tenantToOrg } = await import('./dojo-orgs');
		for (const k of ['client', 'community', 'employer', null, undefined]) {
			const o = tenantToOrg({ ...base, origin: 'organization' } as never, 'admin', k as never);
			expect(o.kind).toBe('Organization');
		}
	});
});
