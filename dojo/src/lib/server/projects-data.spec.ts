// Unit tests for the org Projects read (`projects-data.ts`): the tenant filter +
// envelope, and the count query. Fail-closed on error.
import { describe, it, expect } from 'vitest';
import {
	listOrgProjects,
	listUserProjects,
	getUserProjectConstitution,
	countOrgProjects,
	upsertProjectFromRun,
	fileRunUnderProject,
	AdminError,
	type DojoClient
} from './projects-data';

function makeListDb(result: { data: unknown; error: unknown }) {
	const captured: { tenantEq?: unknown } = {};
	const b: Record<string, unknown> = {};
	b.from = () => b;
	b.select = () => b;
	b.eq = (_c: string, v: unknown) => {
		captured.tenantEq = v;
		return b;
	};
	b.order = () => Promise.resolve(result);
	return { db: b as unknown as DojoClient, captured };
}

function makeCountDb(result: { count: number | null; error: unknown }) {
	const b: Record<string, unknown> = {};
	b.from = () => b;
	b.select = () => b;
	b.eq = () => Promise.resolve(result);
	return b as unknown as DojoClient;
}

describe('listOrgProjects', () => {
	it('returns the tenant rows', async () => {
		const rows = [{ id: 'p1' }, { id: 'p2' }];
		const { db, captured } = makeListDb({ data: rows, error: null });
		expect(await listOrgProjects(db, 't1')).toEqual(rows);
		expect(captured.tenantEq).toBe('t1');
	});
	it('honest-empty [] when data is null', async () => {
		const { db } = makeListDb({ data: null, error: null });
		expect(await listOrgProjects(db, 't1')).toEqual([]);
	});
	it('fails closed (500) on a query error — never a fixture', async () => {
		const { db } = makeListDb({ data: null, error: { message: 'boom' } });
		await expect(listOrgProjects(db, 't1')).rejects.toBeInstanceOf(AdminError);
	});
});

// Captures the column+value of the `.eq` filter so the user-wide read can assert
// it scopes by `user_id` (the filter IS the authorization), not by tenant.
function makeUserDb(result: { data: unknown; error: unknown }) {
	const captured: { col?: string; val?: unknown } = {};
	const b: Record<string, unknown> = {};
	b.from = () => b;
	b.select = () => b;
	b.eq = (c: string, v: unknown) => {
		captured.col = c;
		captured.val = v;
		return b;
	};
	b.order = () => Promise.resolve(result);
	return { db: b as unknown as DojoClient, captured };
}

describe('listUserProjects — user-wide (every dōjō the caller belongs to)', () => {
	it('returns the caller rows, scoped by user_id (the authz), not a tenant', async () => {
		const rows = [{ id: 'p1' }, { id: 'p2' }];
		const { db, captured } = makeUserDb({ data: rows, error: null });
		expect(await listUserProjects(db, 'u1')).toEqual(rows);
		expect(captured.col).toBe('user_id');
		expect(captured.val).toBe('u1');
	});
	it('honest-empty [] when data is null (genuine empty, not a mask)', async () => {
		const { db } = makeUserDb({ data: null, error: null });
		expect(await listUserProjects(db, 'u1')).toEqual([]);
	});
	it('fails closed (AdminError 500) on a query error — never a fabricated list', async () => {
		const { db } = makeUserDb({ data: null, error: { message: 'boom' } });
		await expect(listUserProjects(db, 'u1')).rejects.toBeInstanceOf(AdminError);
	});
});

describe('countOrgProjects', () => {
	it('returns the count', async () => {
		expect(await countOrgProjects(makeCountDb({ count: 9, error: null }), 't1')).toBe(9);
	});
	it('returns 0 when count is null', async () => {
		expect(await countOrgProjects(makeCountDb({ count: null, error: null }), 't1')).toBe(0);
	});
	it('fails closed (500) on a query error', async () => {
		await expect(countOrgProjects(makeCountDb({ count: null, error: { message: 'boom' } }), 't1')).rejects.toBeInstanceOf(AdminError);
	});
});

// A `.maybeSingle()`-terminal stub capturing the two `.eq` filters, for the
// single-project constitution read.
function makeSingleDb(result: { data: unknown; error: unknown }) {
	const captured: { eqs: [string, unknown][] } = { eqs: [] };
	const b: Record<string, unknown> = {};
	b.from = () => b;
	b.select = () => b;
	b.eq = (c: string, v: unknown) => {
		captured.eqs.push([c, v]);
		return b;
	};
	b.maybeSingle = () => Promise.resolve(result);
	return { db: b as unknown as DojoClient, captured };
}

describe('getUserProjectConstitution — the per-project drill-in read', () => {
	it('returns the stored constitution jsonb, scoped by user_id + slug', async () => {
		const constitution = { rules: [{ scope_key: 'organization', title: 'x', enforcement: 'mandatory' }], conflicts: [], locks: 1 };
		const { db, captured } = makeSingleDb({ data: { constitution }, error: null });
		expect(await getUserProjectConstitution(db, 'u1', 'acme/x')).toEqual(constitution);
		expect(captured.eqs).toContainEqual(['user_id', 'u1']);
		expect(captured.eqs).toContainEqual(['slug', 'acme/x']);
	});
	it('returns null when the project has no federated constitution yet (honest-empty)', async () => {
		const { db } = makeSingleDb({ data: { constitution: null }, error: null });
		expect(await getUserProjectConstitution(db, 'u1', 's')).toBeNull();
	});
	it('returns null on a genuine miss (no row for this user+slug)', async () => {
		const { db } = makeSingleDb({ data: null, error: null });
		expect(await getUserProjectConstitution(db, 'u1', 's')).toBeNull();
	});
	it('fails closed (AdminError 500) on a query error — never a fabricated constitution', async () => {
		const { db } = makeSingleDb({ data: null, error: { message: 'boom' } });
		await expect(getUserProjectConstitution(db, 'u1', 's')).rejects.toBeInstanceOf(AdminError);
	});
});

function makeUpsertDb(error: unknown, id = 'proj-1') {
	const captured: { payload?: Record<string, unknown>; conflict?: unknown } = {};
	const b: Record<string, unknown> = {};
	b.from = () => b;
	b.upsert = (payload: Record<string, unknown>, opts: unknown) => {
		captured.payload = payload;
		captured.conflict = opts;
		return b;
	};
	b.select = () => b;
	b.single = () => Promise.resolve({ data: error ? null : { id }, error });
	return { db: b as unknown as DojoClient, captured };
}

const CALLER = { userId: 'u1', tenantId: 't1' };

describe('upsertProjectFromRun', () => {
	it('owns the row by the authenticated caller, on conflict (user_id, slug)', async () => {
		const { db, captured } = makeUpsertDb(null);
		await upsertProjectFromRun(db, CALLER, { slug: 'acme/ledger', name: 'ledger', classification: 'company', phase: 'watch' });
		expect(captured.payload).toMatchObject({ user_id: 'u1', tenant_id: 't1', slug: 'acme/ledger', name: 'ledger', classification: 'company' });
		expect(captured.conflict).toEqual({ onConflict: 'user_id,slug' });
	});
	it('a personal run writes no tenant or classification, so a project sync filed in a tenant is not demoted (#299)', async () => {
		// The relay files an unbound project as `personal`. Sync files the same
		// (user_id, slug) project in the tenant its repository lives in. Writing
		// `personal`/null on conflict would drop it out of that tenant's Projects;
		// omitting both leaves an existing row alone, and a NEW row still gets the
		// column defaults — classification `personal`, no tenant.
		const { db, captured } = makeUpsertDb(null);
		await upsertProjectFromRun(db, CALLER, { slug: 's', name: 'n', classification: 'personal', phase: 'watch' });
		expect(captured.payload).not.toHaveProperty('tenant_id');
		expect(captured.payload).not.toHaveProperty('classification');
	});
	it('a run bound to a tenant still writes its tenant and classification', async () => {
		const { db, captured } = makeUpsertDb(null);
		await upsertProjectFromRun(db, CALLER, { slug: 's', name: 'n', classification: 'client', phase: 'watch' });
		expect(captured.payload).toMatchObject({ tenant_id: 't1', classification: 'client' });
	});
	it('does NOT write phase (insert-only via the column default; a dojo advance is not clobbered)', async () => {
		const { db, captured } = makeUpsertDb(null);
		await upsertProjectFromRun(db, CALLER, { slug: 's', name: 'n', classification: 'client', phase: 'adopt' });
		expect(captured.payload).not.toHaveProperty('phase');
	});
	it('sets last_run_at', async () => {
		const { db, captured } = makeUpsertDb(null);
		await upsertProjectFromRun(db, CALLER, { slug: 's', name: 'n', classification: 'personal', phase: 'watch' });
		expect(typeof captured.payload?.last_run_at).toBe('string');
	});
	it('writes the constitution when the daemon federated one', async () => {
		const { db, captured } = makeUpsertDb(null);
		const constitution = { rules: [{ level: 'company', text: 'x', hard: true }], conflicts: [], locks: 1 };
		await upsertProjectFromRun(db, CALLER, { slug: 's', name: 'n', classification: 'company', phase: 'watch', constitution });
		expect(captured.payload?.constitution).toEqual(constitution);
	});
	it('OMITS constitution when absent (preserved on conflict, like phase — not nulled)', async () => {
		const { db, captured } = makeUpsertDb(null);
		await upsertProjectFromRun(db, CALLER, { slug: 's', name: 'n', classification: 'personal', phase: 'watch' });
		expect(captured.payload).not.toHaveProperty('constitution');
	});
	it('returns the project id, so the run can be filed under it (#252)', async () => {
		const { db } = makeUpsertDb(null, 'proj-9');
		expect(await upsertProjectFromRun(db, CALLER, { slug: 's', name: 'n', classification: 'personal', phase: 'watch' })).toBe('proj-9');
	});
	it('throws AdminError on a DB error (the caller swallows it fire-and-forget)', async () => {
		const { db } = makeUpsertDb({ message: 'boom' });
		await expect(upsertProjectFromRun(db, CALLER, { slug: 's', name: 'n', classification: 'personal', phase: 'watch' })).rejects.toBeInstanceOf(AdminError);
	});
});

// #252: a relay session carries its owner's project, which is what lets a team
// owning that project see THAT the run is going (status only).
describe('fileRunUnderProject', () => {
	it('sets the session\'s project_id, by the session id', async () => {
		const calls: Array<[string, unknown]> = [];
		const b: Record<string, unknown> = {};
		b.from = (t: string) => (calls.push(['from', t]), b);
		b.update = (v: unknown) => (calls.push(['update', v]), b);
		b.eq = (c: string, v: unknown) => (calls.push(['eq', [c, v]]), Promise.resolve({ error: null }));
		await fileRunUnderProject(b as unknown as DojoClient, 'sess-1', 'proj-1');
		expect(calls).toEqual([
			['from', 'relay_sessions'],
			['update', { project_id: 'proj-1' }],
			['eq', ['id', 'sess-1']]
		]);
	});
	it('throws AdminError on a DB error', async () => {
		const b: Record<string, unknown> = {};
		b.from = () => b;
		b.update = () => b;
		b.eq = () => Promise.resolve({ error: { message: 'denied' } });
		await expect(fileRunUnderProject(b as unknown as DojoClient, 's', 'p')).rejects.toBeInstanceOf(AdminError);
	});
});
