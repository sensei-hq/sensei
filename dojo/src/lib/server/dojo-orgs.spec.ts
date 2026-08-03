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

const { listUserOrgs, getUserOrg } = await import('./dojo-orgs');
const { membershipKindToOrgKind, orgKindKanji } = await import('../dojo-data');

const TENANT_ROW = { id: 't1', key: 'gh/acme', org: 'acme', name: 'Acme', self_hosted: false };

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
		expect(orgs[0]).toMatchObject({ id: 't1', url: 'gh/acme', name: 'Acme', role: 'Admin' });
	});

	it('derives the REAL kind + kanji from membership.kind (not the old hardcoded Community)', async () => {
		stub = makeDb({ data: [{ role: 'admin', kind: 'employer', tenant: TENANT_ROW }], error: null });
		const orgs = await listUserOrgs('u1');
		expect(orgs[0].kind).toBe('Employer');
		expect(orgs[0].kanji).toBe('社');
	});

	it('does NOT fabricate counts — members/projects/pending are undefined when not computed', async () => {
		stub = makeDb({ data: [{ role: 'admin', kind: 'client', tenant: TENANT_ROW }], error: null });
		const [org] = await listUserOrgs('u1');
		expect(org.kind).toBe('Client');
		expect(org.members).toBeUndefined();
		expect(org.projects).toBeUndefined();
		expect(org.pending).toBeUndefined();
	});
});

describe('getUserOrg — fail closed on either lookup error', () => {
	it('throws 500 when the tenant lookup errors', async () => {
		stub = makeDb({ data: null, error: { message: 'tenant boom' } });
		const err = await getUserOrg('u1', 'gh/acme').catch((e) => e);
		expect(err?.status).toBe(500);
	});

	it('throws 500 when the membership lookup errors', async () => {
		stub = makeDb({ data: TENANT_ROW, error: null }, { data: null, error: { message: 'mem boom' } });
		const err = await getUserOrg('u1', 'gh/acme').catch((e) => e);
		expect(err?.status).toBe(500);
	});

	it('returns undefined on a genuine tenant miss (no error, no row)', async () => {
		stub = makeDb({ data: null, error: null });
		expect(await getUserOrg('u1', 'gh/none')).toBeUndefined();
	});

	it('returns undefined when the tenant exists but the user is not a member', async () => {
		stub = makeDb({ data: TENANT_ROW, error: null }, { data: null, error: null });
		expect(await getUserOrg('u1', 'gh/acme')).toBeUndefined();
	});

	it('maps to a DojoOrg on a real hit', async () => {
		stub = makeDb({ data: TENANT_ROW, error: null }, { data: { role: 'lead' }, error: null });
		const org = await getUserOrg('u1', 'gh/acme');
		expect(org).toMatchObject({ id: 't1', url: 'gh/acme', role: 'Lead' });
	});

	it('derives the REAL kind from membership.kind on a hit', async () => {
		stub = makeDb(
			{ data: TENANT_ROW, error: null },
			{ data: { role: 'lead', kind: 'community' }, error: null }
		);
		const org = await getUserOrg('u1', 'gh/acme');
		expect(org?.kind).toBe('Community');
		expect(org?.kanji).toBe('群');
	});
});

describe('membershipKindToOrgKind — enum → OrgKind (unknown/missing → Community, never fabricated)', () => {
	it('maps each known kind', () => {
		expect(membershipKindToOrgKind('employer')).toBe('Employer');
		expect(membershipKindToOrgKind('client')).toBe('Client');
		expect(membershipKindToOrgKind('personal')).toBe('Personal');
		expect(membershipKindToOrgKind('community')).toBe('Community');
	});
	it('is case-insensitive', () => {
		expect(membershipKindToOrgKind('EMPLOYER')).toBe('Employer');
	});
	it('falls back to Community on unknown/null/undefined (safe generic bucket)', () => {
		expect(membershipKindToOrgKind('wat')).toBe('Community');
		expect(membershipKindToOrgKind(null)).toBe('Community');
		expect(membershipKindToOrgKind(undefined)).toBe('Community');
	});
});

describe('orgKindKanji — identity glyph per kind', () => {
	it('maps each kind to its ladder kanji', () => {
		expect(orgKindKanji('Employer')).toBe('社');
		expect(orgKindKanji('Client')).toBe('客');
		expect(orgKindKanji('Personal')).toBe('己');
		expect(orgKindKanji('Community')).toBe('群');
	});
});
