// Unit tests for the org Projects read (`projects-data.ts`): the tenant filter +
// envelope, and the count query. Fail-closed on error.
import { describe, it, expect } from 'vitest';
import { listOrgProjects, countOrgProjects, upsertProjectFromRun, AdminError, type DojoClient } from './projects-data';

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

function makeUpsertDb(error: unknown) {
	const captured: { payload?: Record<string, unknown>; conflict?: unknown } = {};
	const b: Record<string, unknown> = {};
	b.from = () => b;
	b.upsert = (payload: Record<string, unknown>, opts: unknown) => {
		captured.payload = payload;
		captured.conflict = opts;
		return Promise.resolve({ error });
	};
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
	it('stores a personal project tenant-less (tenant_id null)', async () => {
		const { db, captured } = makeUpsertDb(null);
		await upsertProjectFromRun(db, CALLER, { slug: 's', name: 'n', classification: 'personal', phase: 'watch' });
		expect(captured.payload?.tenant_id).toBeNull();
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
	it('throws AdminError on a DB error (the caller swallows it fire-and-forget)', async () => {
		const { db } = makeUpsertDb({ message: 'boom' });
		await expect(upsertProjectFromRun(db, CALLER, { slug: 's', name: 'n', classification: 'personal', phase: 'watch' })).rejects.toBeInstanceOf(AdminError);
	});
});
