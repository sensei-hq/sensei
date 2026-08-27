// Unit tests for the admin-console store logic (`admin-data.ts`). Exercises:
//   • listMembers / listIdentities / listPolicies / listAudit — the tenant
//     filter + envelope rows + error → AdminError(500), and the audit limit
//     clamp.
//   • getHealth — the four isolated count queries composed into the rollup, and
//     a count-query error surfacing (never a silent 0).
import { describe, it, expect } from 'vitest';
import {
	listMembers,
	listIdentities,
	listPolicies,
	listAudit,
	getHealth,
	setMemberRole,
	addMember,
	parseNewMember,
	upsertPolicy,
	parseUpsertPolicy,
	patchPolicy,
	parsePatchPolicy,
	deletePolicy,
	createIdentity,
	parseNewIdentity,
	updateIdentity,
	parsePatchIdentity,
	deleteIdentity,
	slugify,
	parseNewDojo,
	createDojo,
	AdminError,
	type DojoClient
} from './admin-data';

type Terminal = { data?: unknown; count?: number | null; error: unknown };

/** A stub whose terminal (`.order()` / `.limit()`) resolves the given result. */
function makeListDb(result: Terminal) {
	const captured: { limit?: number } = {};
	const b: Record<string, unknown> = {};
	b.from = () => b;
	b.select = () => b;
	b.eq = () => b;
	b.order = () => b;
	b.limit = (n: number) => {
		captured.limit = n;
		return Promise.resolve(result);
	};
	// list*() without a limit awaits `.order()` directly.
	b.then = (resolve: (v: Terminal) => unknown) => resolve(result);
	return { db: b as unknown as DojoClient, captured };
}

describe('listPolicies', () => {
	it('returns the rows on success', async () => {
		const rows = [{ id: 'm1' }];
		expect(await listPolicies(makeListDb({ data: rows, error: null }).db, 't1')).toEqual(rows);
	});
	it('returns [] when data is null', async () => {
		expect(await listPolicies(makeListDb({ data: null, error: null }).db, 't1')).toEqual([]);
	});
	it('throws AdminError(500) on a query error', async () => {
		await expect(listPolicies(makeListDb({ data: null, error: { message: 'boom' } }).db, 't1')).rejects.toThrow(AdminError);
	});
});

// ── listMembers: memberships query + WS-1 identity enrichment (a second query) ──
// listMembers issues TWO queries: memberships (terminal `.order()`) then
// dojo.identities via resolveDisplayNames (terminal `.in()`).
function makeMembersDb(members: Terminal, identities: { data?: unknown; error: unknown }) {
	const b: Record<string, unknown> = {};
	b.from = () => b;
	b.select = () => b;
	b.eq = () => b;
	b.order = () => Promise.resolve(members);
	b.in = () => Promise.resolve(identities);
	return b as unknown as DojoClient;
}

describe('listMembers (WS-1 identity enrichment)', () => {
	it('enriches each member with the display name + email resolved from identities', async () => {
		const db = makeMembersDb(
			{ data: [{ id: 'm1', user_id: 'u1' }, { id: 'm2', user_id: 'u2' }], error: null },
			{ data: [{ principal_id: 'u1', display_name: 'Ada', email: 'ada@x.co', last_login_at: null }], error: null }
		);
		const out = await listMembers(db, 't1');
		expect(out[0]).toMatchObject({ id: 'm1', display_name: 'Ada', email: 'ada@x.co' });
		expect(out[1]).toMatchObject({ id: 'm2', display_name: null, email: null }); // no identity → shortId fallback
	});
	it('returns [] (and skips the identity query) when there are no members', async () => {
		const db = makeMembersDb({ data: null, error: null }, { data: [], error: null });
		expect(await listMembers(db, 't1')).toEqual([]);
	});
	it('throws AdminError(500) when the memberships query errors', async () => {
		const db = makeMembersDb({ data: null, error: { message: 'boom' } }, { data: [], error: null });
		await expect(listMembers(db, 't1')).rejects.toThrow(AdminError);
	});
	it('throws AdminError(500) when the identity-enrichment query errors (fail-closed)', async () => {
		const db = makeMembersDb({ data: [{ id: 'm1', user_id: 'u1' }], error: null }, { data: null, error: { message: 'id boom' } });
		await expect(listMembers(db, 't1')).rejects.toThrow(AdminError);
	});
});

describe('listAudit', () => {
	it('clamps the limit to 1..500 (default 100)', async () => {
		const d1 = makeListDb({ data: [], error: null });
		await listAudit(d1.db, 't1');
		expect(d1.captured.limit).toBe(100);
		const d2 = makeListDb({ data: [], error: null });
		await listAudit(d2.db, 't1', 9999);
		expect(d2.captured.limit).toBe(500);
		const d3 = makeListDb({ data: [], error: null });
		await listAudit(d3.db, 't1', 0);
		expect(d3.captured.limit).toBe(1);
	});
});

// ── getHealth: four sequential count queries ─────────────────────────────────
// Each `.from(...).select(..,{head}).eq(...).gte()/.in()/.eq()` chain resolves
// (awaited) to the next `{ count, error }` in order.
function makeCountDb(results: Terminal[]) {
	let i = 0;
	const next = (): Terminal => results[i++] ?? { count: 0, error: null };
	const b: Record<string, unknown> = {};
	b.from = () => b;
	b.select = () => b;
	b.eq = () => b;
	b.in = () => b;
	b.gte = () => Promise.resolve(next());
	// The error_rate query ends on `.eq()` (no gte/in) — resolve on await.
	b.then = (resolve: (v: Terminal) => unknown) => resolve(next());
	return b as unknown as DojoClient;
}

describe('getHealth', () => {
	it('composes the four counts into the rollup', async () => {
		const db = makeCountDb([
			{ count: 0, error: null }, // memberships scope fetch (.eq → .then; data unused, ids => [])
			{ count: 3, error: null }, // connections (.gte heartbeat)
			{ count: 12, error: null }, // queue_depth (.eq state) — resolves via .then
			{ count: 5, error: null }, // publish_rate (.gte ts)
			{ count: 1, error: null } // error_rate (.eq sync_status) — via .then
		]);
		const h = await getHealth(db, 't1');
		expect(h).toEqual({ connections: 3, queue_depth: 12, publish_rate_1h: 5, error_rate_1h: 1 });
	});
	it('surfaces a count-query error rather than blanking the strip', async () => {
		const db = makeCountDb([{ count: null, error: { message: 'count boom' } }]);
		await expect(getHealth(db, 't1')).rejects.toThrow(AdminError);
	});
});

// ── writes ───────────────────────────────────────────────────────────────────
// A stub that captures the mutated table + payload, then resolves the terminal
// (`.maybeSingle()` / `.single()`, or awaiting `.select()` for a delete). All the
// intermediate chain links (update/insert/upsert/delete/eq/select) return `b`.
type MutTerminal = { data?: unknown; error: unknown };
function makeMutDb(result: MutTerminal) {
	const captured: { table?: string; op?: string; payload?: unknown; conflict?: unknown } = {};
	const b: Record<string, unknown> = {};
	b.from = (t: string) => {
		captured.table = t;
		return b;
	};
	b.update = (payload: unknown) => {
		captured.op = 'update';
		captured.payload = payload;
		return b;
	};
	b.insert = (payload: unknown) => {
		captured.op = 'insert';
		captured.payload = payload;
		return b;
	};
	b.upsert = (payload: unknown, opts: unknown) => {
		captured.op = 'upsert';
		captured.payload = payload;
		captured.conflict = opts;
		return b;
	};
	b.delete = () => {
		captured.op = 'delete';
		return b;
	};
	b.eq = () => b;
	b.select = () => b;
	b.maybeSingle = () => Promise.resolve(result);
	b.single = () => Promise.resolve(result);
	// A delete awaits `.select()` directly (no single).
	b.then = (resolve: (v: MutTerminal) => unknown) => resolve(result);
	return { db: b as unknown as DojoClient, captured };
}

describe('setMemberRole', () => {
	it('rejects an unknown role with 400 before touching the db', async () => {
		const { db } = makeMutDb({ data: null, error: null });
		await expect(setMemberRole(db, 't1', 'u1', 'wizard')).rejects.toMatchObject({ status: 400 });
	});
	it('updates the role and returns { user_id, role }', async () => {
		const { db, captured } = makeMutDb({ data: { user_id: 'u1', role: 'lead' }, error: null });
		expect(await setMemberRole(db, 't1', 'u1', 'lead')).toEqual({ user_id: 'u1', role: 'lead' });
		expect(captured.op).toBe('update');
		expect((captured.payload as { role: string }).role).toBe('lead');
	});
	it('404s when no membership matches', async () => {
		const { db } = makeMutDb({ data: null, error: null });
		await expect(setMemberRole(db, 't1', 'ghost', 'admin')).rejects.toMatchObject({ status: 404 });
	});
});

describe('parseNewMember', () => {
	it('requires user_id, a known kind, and a known auth method', () => {
		expect(() => parseNewMember({})).toThrow(AdminError);
		expect(() => parseNewMember({ user_id: 'u1', kind: 'nope', authenticated_via: 'sso' })).toThrow();
		expect(() => parseNewMember({ user_id: 'u1', kind: 'client', authenticated_via: 'x' })).toThrow();
	});
	it('defaults role to contributor and prefers an explicit role', () => {
		expect(parseNewMember({ user_id: 'u1', kind: 'client', authenticated_via: 'sso' }).role).toBe('contributor');
		expect(
			parseNewMember({ user_id: 'u1', kind: 'client', authenticated_via: 'sso', role: 'lead' }).role
		).toBe('lead');
		expect(
			parseNewMember({ user_id: 'u1', kind: 'client', authenticated_via: 'sso', git_provider_role: 'maintainer' }).role
		).toBe('maintainer');
	});
});

describe('addMember', () => {
	it('inserts the membership with the tenant and returns { id, role } (no dojo_url — derived from the tenant)', async () => {
		const { db, captured } = makeMutDb({ data: { id: 'm1', role: 'contributor' }, error: null });
		const input = parseNewMember({ user_id: 'u1', kind: 'client', authenticated_via: 'sso' });
		expect(await addMember(db, 't1', input)).toEqual({ id: 'm1', role: 'contributor' });
		const payload = captured.payload as Record<string, unknown>;
		expect(payload.tenant_id).toBe('t1');
		expect(payload.user_id).toBe('u1');
		expect(payload).not.toHaveProperty('dojo_url'); // Rule C: derived from dojo.tenants.dojo_url
	});
	it('maps a unique-violation (23505) to 409', async () => {
		const { db } = makeMutDb({ data: null, error: { code: '23505', message: 'dup' } });
		const input = parseNewMember({ user_id: 'u1', kind: 'client', authenticated_via: 'sso' });
		await expect(addMember(db, 't1', input)).rejects.toMatchObject({ status: 409 });
	});
});

describe('policies write', () => {
	it('parseUpsertPolicy requires scope_key + validates enums', () => {
		expect(() => parseUpsertPolicy({})).toThrow(AdminError);
		expect(() => parseUpsertPolicy({ scope_key: 's', attribution_default: 'x' })).toThrow();
		expect(() => parseUpsertPolicy({ scope_key: 's', retention_days: -1 })).toThrow();
		expect(parseUpsertPolicy({ scope_key: 's', retention_days: 30 })).toEqual({ scope_key: 's', retention_days: 30 });
	});
	it('upsertPolicy upserts on tenant_id,scope_key and returns { id, scope_key }', async () => {
		const { db, captured } = makeMutDb({ data: { id: 'p1', scope_key: 'all-org' }, error: null });
		const out = await upsertPolicy(db, 't1', parseUpsertPolicy({ scope_key: 'all-org' }));
		expect(out).toEqual({ id: 'p1', scope_key: 'all-org' });
		expect(captured.op).toBe('upsert');
		expect(captured.conflict).toEqual({ onConflict: 'tenant_id,scope_key' });
	});
	it('patchPolicy 404s when nothing matched', async () => {
		const { db } = makeMutDb({ data: null, error: null });
		await expect(patchPolicy(db, 't1', 'p9', parsePatchPolicy({ retention_days: 90 }))).rejects.toMatchObject({ status: 404 });
	});
	it('deletePolicy is true when a row was removed, false when none', async () => {
		expect(await deletePolicy(makeMutDb({ data: [{ id: 'p1' }], error: null }).db, 't1', 'p1')).toBe(true);
		expect(await deletePolicy(makeMutDb({ data: [], error: null }).db, 't1', 'p9')).toBe(false);
	});
});

// ── identities: GLOBAL rows, tenant-scoped routes ───────────────────────────
// dojo.identities has no tenant_id (it says "this GitHub account is this
// person", which is not a per-tenant fact) and keys on principal_id. The routes
// are still tenant-addressed at the admin floor, so the isolation the dropped
// tenant_id filter used to provide is now an EXPLICIT membership check. These
// tests exist mostly to pin that check: without it, an admin of tenant A could
// read, rewrite or delete the identities of people in tenant B.
//
// Query sequence per function is declared by the queue.
function makeIdentityDb(...results: Terminal[]) {
	const queue = [...results];
	const captured: Record<string, unknown> = { tables: [] as string[] };
	const next = () => Promise.resolve(queue.shift() ?? { data: null, error: null });
	const b: Record<string, unknown> = {};
	b.from = (t: string) => {
		(captured.tables as string[]).push(t);
		return b;
	};
	b.select = () => b;
	b.eq = () => b;
	b.delete = () => b;
	b.insert = (payload: unknown) => {
		captured.payload = payload;
		return b;
	};
	b.update = (payload: unknown) => {
		captured.payload = payload;
		return b;
	};
	b.in = (col: string, vals: unknown) => {
		captured.inCol = col;
		captured.inVals = vals;
		return b;
	};
	b.order = next;
	b.maybeSingle = next;
	b.single = next;
	b.then = (resolve: (v: Terminal) => unknown) => next().then(resolve);
	return { db: b as unknown as DojoClient, captured };
}

const MEMBERS_OF_T1: Terminal = { data: [{ user_id: 'p1' }, { user_id: 'p2' }], error: null };

describe('listIdentities (global rows, scoped by membership)', () => {
	it('scopes to the principals who are members of the tenant', async () => {
		// The assertion that matters: the filter is `principal_id IN (this
		// tenant's members)`. A regression to an unfiltered read would still
		// return rows and still look fine on screen.
		const rows = [{ id: 'i1', principal_id: 'p1' }];
		const { db, captured } = makeIdentityDb(MEMBERS_OF_T1, { data: rows, error: null });
		expect(await listIdentities(db, 't1')).toEqual(rows);
		expect(captured.inCol).toBe('principal_id');
		expect(captured.inVals).toEqual(['p1', 'p2']);
		expect(captured.tables).toEqual(['memberships', 'identities']);
	});

	it('returns [] for a member-less tenant without querying identities at all', async () => {
		// Genuinely empty, not a masked failure: no members means no identities
		// to show. Skipping the second query also avoids `.in(col, [])`.
		const { db, captured } = makeIdentityDb({ data: [], error: null });
		expect(await listIdentities(db, 'empty')).toEqual([]);
		expect(captured.tables).toEqual(['memberships']);
	});

	it('throws AdminError(500) when the membership scope query errors', async () => {
		const { db } = makeIdentityDb({ data: null, error: { message: 'boom' } });
		await expect(listIdentities(db, 't1')).rejects.toMatchObject({ status: 500 });
	});

	it('throws AdminError(500) when the identities query errors', async () => {
		const { db } = makeIdentityDb(MEMBERS_OF_T1, { data: null, error: { message: 'boom' } });
		await expect(listIdentities(db, 't1')).rejects.toMatchObject({ status: 500 });
	});
});

describe('identities write', () => {
	it('parseNewIdentity requires principal_id + a known provider + subject', () => {
		expect(() => parseNewIdentity({})).toThrow(AdminError);
		expect(() => parseNewIdentity({ principal_id: 'p1', provider: 'x', subject: 's' })).toThrow();
		expect(() => parseNewIdentity({ principal_id: 'p1', provider: 'sso', subject: '' })).toThrow();
		// user_id is the OLD field name and must not be silently accepted — doing
		// so would write a null principal_id and violate the NOT NULL at runtime.
		expect(() => parseNewIdentity({ user_id: 'p1', provider: 'sso', subject: 'sub' })).toThrow();
		expect(parseNewIdentity({ principal_id: 'p1', provider: 'sso', subject: 'sub' })).toMatchObject({
			principal_id: 'p1',
			provider: 'sso',
			subject: 'sub'
		});
	});

	it('createIdentity refuses a principal who is not a member of the tenant', async () => {
		// The replacement for the dropped tenant_id filter. Without it an admin
		// could attach an identity to anyone in any tenant.
		const { db, captured } = makeIdentityDb({ data: null, error: null }); // membership miss
		await expect(
			createIdentity(db, 't1', parseNewIdentity({ principal_id: 'outsider', provider: 'sso', subject: 's' }))
		).rejects.toMatchObject({ status: 404 });
		expect(captured.payload).toBeUndefined();
	});

	it('createIdentity inserts principal_id and no tenant_id for a real member', async () => {
		const { db, captured } = makeIdentityDb(
			{ data: { id: 'm1' }, error: null }, // membership hit
			{ data: { id: 'i1' }, error: null }
		);
		const out = await createIdentity(
			db,
			't1',
			parseNewIdentity({ principal_id: 'p1', provider: 'sso', subject: 's' })
		);
		expect(out).toEqual({ id: 'i1' });
		const payload = captured.payload as Record<string, unknown>;
		expect(payload.principal_id).toBe('p1');
		expect(payload).not.toHaveProperty('tenant_id'); // the column does not exist
		expect(payload).not.toHaveProperty('user_id');
	});

	it('createIdentity maps a unique-violation to 409', async () => {
		const { db } = makeIdentityDb(
			{ data: { id: 'm1' }, error: null },
			{ data: null, error: { code: '23505', message: 'dup' } }
		);
		await expect(
			createIdentity(db, 't1', parseNewIdentity({ principal_id: 'p1', provider: 'sso', subject: 's' }))
		).rejects.toMatchObject({ status: 409 });
	});

	it('updateIdentity 404s for an identity belonging to another tenant', async () => {
		// Reads the identity's principal, then checks membership. Both misses
		// answer 404 "no such identity" — an admin of tenant A must not learn
		// that the id exists in tenant B.
		const { db, captured } = makeIdentityDb(
			{ data: { principal_id: 'outsider' }, error: null }, // identity exists…
			{ data: null, error: null } // …but not in this tenant
		);
		const err = await updateIdentity(db, 't1', 'i9', parsePatchIdentity({ email: 'a@b.co' })).catch(
			(e) => e
		);
		expect(err.status).toBe(404);
		expect(err.message).toBe('no such identity');
		expect(captured.payload).toBeUndefined();
	});

	it('updateIdentity 404s when the identity does not exist', async () => {
		const { db } = makeIdentityDb({ data: null, error: null });
		await expect(
			updateIdentity(db, 't1', 'i9', parsePatchIdentity({ email: 'a@b.co' }))
		).rejects.toMatchObject({ status: 404 });
	});

	it('updateIdentity patches a member identity; parsePatchIdentity rejects non-strings', async () => {
		expect(() => parsePatchIdentity({ email: 123 as unknown as string })).toThrow(AdminError);
		const { db } = makeIdentityDb(
			{ data: { principal_id: 'p1' }, error: null },
			{ data: { id: 'm1' }, error: null },
			{ data: { id: 'i1' }, error: null }
		);
		expect(await updateIdentity(db, 't1', 'i1', parsePatchIdentity({ email: 'a@b.co' }))).toEqual({
			id: 'i1'
		});
	});

	it('deleteIdentity refuses an identity outside the tenant, and reports rows removed', async () => {
		const outside = makeIdentityDb(
			{ data: { principal_id: 'outsider' }, error: null },
			{ data: null, error: null }
		);
		await expect(deleteIdentity(outside.db, 't1', 'i9')).rejects.toMatchObject({ status: 404 });

		const removed = makeIdentityDb(
			{ data: { principal_id: 'p1' }, error: null },
			{ data: { id: 'm1' }, error: null },
			{ data: [{ id: 'i1' }], error: null }
		);
		expect(await deleteIdentity(removed.db, 't1', 'i1')).toBe(true);

		const none = makeIdentityDb(
			{ data: { principal_id: 'p1' }, error: null },
			{ data: { id: 'm1' }, error: null },
			{ data: [], error: null }
		);
		expect(await deleteIdentity(none.db, 't1', 'i2')).toBe(false);
	});
});

// A two-insert stub for createDojo: `.single()` shifts the next queued result
// (tenant insert, then the addMember membership insert). Captures each insert's
// table + payload so the tenant shape + creator admin membership can be asserted.
function makeCreateDb(...results: { data?: unknown; error: unknown }[]) {
	const queue = [...results];
	const inserts: { table?: string; payload?: Record<string, unknown> }[] = [];
	let table: string | undefined;
	const b: Record<string, unknown> = {};
	b.from = (t: string) => {
		table = t;
		return b;
	};
	b.insert = (payload: Record<string, unknown>) => {
		inserts.push({ table, payload });
		return b;
	};
	b.select = () => b;
	b.single = () => Promise.resolve(queue.shift() ?? { data: null, error: null });
	return { db: b as unknown as DojoClient, inserts };
}

describe('slugify', () => {
	it('lowercases, hyphenates non-alphanumerics, trims edges', () => {
		expect(slugify('Acme Corp!')).toBe('acme-corp');
		expect(slugify('  Hello  World  ')).toBe('hello-world');
		expect(slugify('已经 rust')).toBe('rust');
	});
	it('is empty when nothing survives', () => {
		expect(slugify('!!!')).toBe('');
	});
});

describe('parseNewDojo', () => {
	it('requires a name that slugifies + a valid kind', () => {
		expect(() => parseNewDojo({})).toThrow();
		expect(() => parseNewDojo({ name: '!!!', kind: 'client' })).toThrow();
		expect(() => parseNewDojo({ name: 'Acme', kind: 'wizard' })).toThrow();
		expect(parseNewDojo({ name: 'Acme', kind: 'client' })).toEqual({ name: 'Acme', kind: 'client' });
	});
});

describe('createDojo', () => {
	it('inserts an org tenant (key org/{slug}) and makes the creator admin', async () => {
		const { db, inserts } = makeCreateDb(
			{ data: { id: 't-new', key: 'org/acme', name: 'Acme' }, error: null }, // tenant insert
			{ data: { id: 'm-new', role: 'admin' }, error: null } // addMember insert
		);
		const out = await createDojo(db, 'u1', { name: 'Acme', kind: 'employer' });
		expect(out).toEqual({ id: 't-new', key: 'org/acme', name: 'Acme' });
		// tenant shape
		expect(inserts[0]).toMatchObject({
			table: 'tenants',
			payload: { key: 'org/acme', origin: 'org', org: 'acme', name: 'Acme', scope: 'private' }
		});
		// creator admin membership
		expect(inserts[1]).toMatchObject({
			table: 'memberships',
			payload: { tenant_id: 't-new', user_id: 'u1', role: 'admin', kind: 'employer' }
		});
	});
	it('maps a key collision (23505) to 409', async () => {
		const { db } = makeCreateDb({ data: null, error: { code: '23505', message: 'dup key' } });
		await expect(createDojo(db, 'u1', { name: 'Acme', kind: 'client' })).rejects.toMatchObject({ status: 409 });
	});
});
