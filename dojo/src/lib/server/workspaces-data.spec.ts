// Unit tests for the workspaces data layer (`workspaces-data.ts`, #299): the tenant
// list, the caller's RLS-scoped list, the drill-in by id, and a run attaching to
// the shared workspace. Fail-closed on error.
import { describe, it, expect } from 'vitest';
import {
	listOrgWorkspaces,
	listUserWorkspaces,
	getWorkspaceConstitution,
	attachRunToWorkspace,
	fileRunUnderWorkspace,
	AdminError,
	type DojoClient
} from './workspaces-data';

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

describe('listOrgWorkspaces', () => {
	it('returns the tenant rows', async () => {
		const rows = [{ id: 'p1' }, { id: 'p2' }];
		const { db, captured } = makeListDb({ data: rows, error: null });
		expect(await listOrgWorkspaces(db, 't1')).toEqual(rows);
		expect(captured.tenantEq).toBe('t1');
	});
	it('honest-empty [] when data is null', async () => {
		const { db } = makeListDb({ data: null, error: null });
		expect(await listOrgWorkspaces(db, 't1')).toEqual([]);
	});
	it('fails closed (500) on a query error — never a fixture', async () => {
		const { db } = makeListDb({ data: null, error: { message: 'boom' } });
		await expect(listOrgWorkspaces(db, 't1')).rejects.toBeInstanceOf(AdminError);
	});
});

// #299: a workspace is ONE SHARED item in a dōjō. The caller's list and the
// drill-in read go through RLS (dojo.can_read_workspace — whoever may read one of
// its repositories), never a `user_id` filter: there is no owner column.
function makeUserDb(result: { data: unknown; error: unknown }) {
	const captured: { eqs: [string, unknown][] } = { eqs: [] };
	const b: Record<string, unknown> = {};
	b.from = () => b;
	b.select = () => b;
	b.eq = (c: string, v: unknown) => {
		captured.eqs.push([c, v]);
		return b;
	};
	b.order = () => Promise.resolve(result);
	return { db: b as unknown as DojoClient, captured };
}

describe('listUserWorkspaces — every workspace the caller may read (RLS), across dōjōs', () => {
	it('returns the readable rows with their dōjō, with no owner filter', async () => {
		const rows = [{ id: 'p1', tenant_id: 't1' }, { id: 'p2', tenant_id: 't2' }];
		const { db, captured } = makeUserDb({ data: rows, error: null });
		expect(await listUserWorkspaces(db)).toEqual(rows);
		expect(captured.eqs).toEqual([]);
	});
	it('honest-empty [] when data is null (genuine empty, not a mask)', async () => {
		const { db } = makeUserDb({ data: null, error: null });
		expect(await listUserWorkspaces(db)).toEqual([]);
	});
	it('fails closed (AdminError 500) on a query error — never a fabricated list', async () => {
		const { db } = makeUserDb({ data: null, error: { message: 'boom' } });
		await expect(listUserWorkspaces(db)).rejects.toBeInstanceOf(AdminError);
	});
});

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

describe('getWorkspaceConstitution — the per-workspace drill-in read, by id', () => {
	// By id: a slug is unique only within one dōjō, so two of the caller's dōjōs
	// can hold the same slug.
	it('returns the stored constitution jsonb for that workspace id', async () => {
		const constitution = { rules: [{ scope_key: 'organization', title: 'x', enforcement: 'mandatory' }], conflicts: [], locks: 1 };
		const { db, captured } = makeSingleDb({ data: { constitution }, error: null });
		expect(await getWorkspaceConstitution(db, 'p1')).toEqual(constitution);
		expect(captured.eqs).toEqual([['id', 'p1']]);
	});
	it('returns null when the workspace has no federated constitution yet (honest-empty)', async () => {
		const { db } = makeSingleDb({ data: { constitution: null }, error: null });
		expect(await getWorkspaceConstitution(db, 'p1')).toBeNull();
	});
	it('returns null when the caller cannot read the workspace (RLS hides it)', async () => {
		const { db } = makeSingleDb({ data: null, error: null });
		expect(await getWorkspaceConstitution(db, 'p1')).toBeNull();
	});
	it('fails closed (AdminError 500) on a query error — never a fabricated constitution', async () => {
		const { db } = makeSingleDb({ data: null, error: { message: 'boom' } });
		await expect(getWorkspaceConstitution(db, 'p1')).rejects.toBeInstanceOf(AdminError);
	});
});

function makeRpcDb(result: { data: unknown; error: unknown }) {
	const captured: { fn?: string; args?: Record<string, unknown> } = {};
	const db = {
		rpc: (fn: string, args: Record<string, unknown>) => {
			captured.fn = fn;
			captured.args = args;
			return Promise.resolve(result);
		}
	};
	return { db: db as unknown as DojoClient, captured };
}

describe('attachRunToWorkspace — a run attaches to the shared workspace, never creates one', () => {
	it('asks the database for the (tenant, slug) workspace and passes the constitution', async () => {
		const { db, captured } = makeRpcDb({ data: 'proj-1', error: null });
		expect(await attachRunToWorkspace(db, 't1', 'sensei', { rules: [] })).toBe('proj-1');
		expect(captured).toEqual({
			fn: 'attach_run_to_workspace',
			args: { p_tenant_id: 't1', p_slug: 'sensei', p_constitution: { rules: [] } }
		});
	});
	it('passes no constitution when the daemon resolved none', async () => {
		const { db, captured } = makeRpcDb({ data: 'proj-1', error: null });
		await attachRunToWorkspace(db, 't1', 'sensei', undefined);
		expect(captured.args?.p_constitution).toBeNull();
	});
	it('returns null when the dōjō has no such workspace yet (the sync has not filed it)', async () => {
		const { db } = makeRpcDb({ data: null, error: null });
		expect(await attachRunToWorkspace(db, 't1', 'sensei', null)).toBeNull();
	});
	it('fails closed (AdminError 500) on an error', async () => {
		const { db } = makeRpcDb({ data: null, error: { message: 'not a member of this dōjō' } });
		await expect(attachRunToWorkspace(db, 't1', 'sensei', null)).rejects.toBeInstanceOf(AdminError);
	});
});


// #252: a relay session carries its owner's workspace, which is what lets a team
// owning that workspace see THAT the run is going (status only).
describe('fileRunUnderWorkspace', () => {
	it('sets the session\'s workspace_id, by the session id', async () => {
		const calls: Array<[string, unknown]> = [];
		const b: Record<string, unknown> = {};
		b.from = (t: string) => (calls.push(['from', t]), b);
		b.update = (v: unknown) => (calls.push(['update', v]), b);
		b.eq = (c: string, v: unknown) => (calls.push(['eq', [c, v]]), Promise.resolve({ error: null }));
		await fileRunUnderWorkspace(b as unknown as DojoClient, 'sess-1', 'proj-1');
		expect(calls).toEqual([
			['from', 'relay_sessions'],
			['update', { workspace_id: 'proj-1' }],
			['eq', ['id', 'sess-1']]
		]);
	});
	it('throws AdminError on a DB error', async () => {
		const b: Record<string, unknown> = {};
		b.from = () => b;
		b.update = () => b;
		b.eq = () => Promise.resolve({ error: { message: 'denied' } });
		await expect(fileRunUnderWorkspace(b as unknown as DojoClient, 's', 'p')).rejects.toBeInstanceOf(AdminError);
	});
});
