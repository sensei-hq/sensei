// Unit tests for the org Projects read (`projects-data.ts`): the tenant filter +
// envelope, and the count query. Fail-closed on error.
import { describe, it, expect } from 'vitest';
import { listOrgProjects, countOrgProjects, AdminError, type DojoClient } from './projects-data';

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
