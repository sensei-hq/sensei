// The scheduled Azure DevOps member read (#308 R5b, D44): for each connected
// organization whose grant still works, refresh it, read the members and apply
// #309's rule (a complete read ends departed provisioned members; a partial read
// ends nobody). Bounded per run — the connections read longest ago first — and
// one connection's failure never stops the others: it is recorded on the
// connection (members_read_at, members_read_error), never swallowed.
import { describe, it, expect } from 'vitest';
import { fakeDojoDb } from './fake-dojo-db';
import { fakeFetch } from './fake-forge-fetch';
import type { AzureAdapter } from './forge-azure';
import { ForgeReadError } from './forge-github';
import { MEMBER_SYNC_BATCH, runAzureMemberSync } from './azure-member-sync';

const CFG = { clientId: 'client-id-0000', clientSecret: 'client-secret-0000' };
const TOKEN_URL = 'https://login.microsoftonline.com/organizations/oauth2/v2.0/token';
const NOW = 1_800_000_000_000;

const conn = (n: number, over: Record<string, unknown> = {}) => ({
	id: `c-${n}`,
	tenant_id: `t-${n}`,
	provider: 'azure_devops',
	external_id: `org-${n}`,
	external_slug: `org-name-${n}`,
	token_secret_id: `sec-${n}`,
	token_state: 'ok',
	members_read_at: null,
	members_read_error: null,
	...over
});

function adapterFor(fail: Record<string, unknown> = {}, notAdmin: string[] = []) {
	const read: string[] = [];
	const proved: string[] = [];
	const make = (org: string): AzureAdapter =>
		({
			provider: 'azure_devops',
			organization: org,
			connection: async () => ({ orgId: org.replace('org-name-', 'org-'), user: { id: 'u', descriptor: 'aad.x', displayName: null, active: true } }),
			proveAdmin: async () => {
				proved.push(org);
				return !notAdmin.includes(org);
			},
			listMembers: async () => {
				read.push(org);
				if (fail[org]) throw fail[org];
				return { members: [{ id: 'a0e10000-0000-4000-8000-000000000001', login: 'dev@example.com' }], complete: true };
			}
		}) as unknown as AzureAdapter;
	return { make, read, proved };
}

function db(rows: Record<string, unknown>[], writeError?: { message: string }) {
	return fakeDojoDb({
		tenant_connections: { rows, writeError },
		'rpc:forge_token': { rows: ['ztest-refresh-0' as never] },
		'rpc:rotate_forge_token': { rows: [] },
		'rpc:mark_forge_token_rejected': { rows: [] },
		'rpc:reconcile_unit_members': { rows: [{ ended: 0, joined: 0 }] }
	});
}

const entra = () => fakeFetch({ [TOKEN_URL]: { body: { access_token: 'ztest-access', refresh_token: 'ztest-refresh-1', scope: '' } } });

describe('runAzureMemberSync', () => {
	it('reads each working connection, applies the reconcile, and records the read', async () => {
		const d = db([conn(1), conn(2), conn(3, { token_state: 'needs_new_token' }), conn(4, { provider: 'github' }), conn(5, { token_secret_id: null, token_state: null })]);
		const a = adapterFor();
		const out = await runAzureMemberSync(d as never, { cfg: CFG, fetchImpl: entra().fn, adapterFor: a.make, now: NOW });
		expect(a.read).toEqual(['org-name-1', 'org-name-2']);
		expect(d.rpcCalls.filter((c) => c.fn === 'reconcile_unit_members').map((c) => c.args.p_unit)).toEqual(['org-1', 'org-2']);
		expect(out).toEqual({
			read: 2,
			results: [
				{ organization: 'org-name-1', members: { complete: true, listed: 1, ended: 0, joined: 0 } },
				{ organization: 'org-name-2', members: { complete: true, listed: 1, ended: 0, joined: 0 } }
			]
		});
		const at = new Date(NOW).toISOString();
		expect(d.tables.tenant_connections.rows.filter((r) => r.members_read_at === at).map((r) => r.id)).toEqual(['c-1', 'c-2']);
		expect(JSON.stringify(out)).not.toContain('ztest');
	});

	it("one connection's failure is recorded and never stops the others", async () => {
		const d = db([conn(1), conn(2), conn(3)]);
		const a = adapterFor({ 'org-name-2': new ForgeReadError(503, 'Azure DevOps read failed (503) for vsaex.dev.azure.com/org-name-2/_apis/userentitlements') });
		const out = await runAzureMemberSync(d as never, { cfg: CFG, fetchImpl: entra().fn, adapterFor: a.make, now: NOW });
		expect(a.read).toEqual(['org-name-1', 'org-name-2', 'org-name-3']);
		expect(out.results[1]).toEqual({ organization: 'org-name-2', members: { error: 'Azure DevOps could not list the members (503); nobody was ended' } });
		const failed = d.tables.tenant_connections.rows.find((r) => r.id === 'c-2');
		expect(failed?.members_read_error).toBe('Azure DevOps could not list the members (503); nobody was ended');
		expect(d.rpcCalls.filter((c) => c.fn === 'reconcile_unit_members')).toHaveLength(2);
	});

	it('a grant Microsoft refuses is marked "reconnect", recorded, and the run goes on', async () => {
		const d = db([conn(1), conn(2)]);
		const tokens = fakeFetch({ [TOKEN_URL]: { status: 400, body: { error: 'invalid_grant' } } });
		const out = await runAzureMemberSync(d as never, { cfg: CFG, fetchImpl: tokens.fn, adapterFor: adapterFor().make, now: NOW });
		expect(d.rpcCalls.filter((c) => c.fn === 'mark_forge_token_rejected')).toHaveLength(2);
		expect(out.results.map((r) => ('error' in r ? r.error : null))).toEqual([
			'Microsoft no longer accepts this Azure DevOps connection — reconnect Azure DevOps',
			'Microsoft no longer accepts this Azure DevOps connection — reconnect Azure DevOps'
		]);
		expect(d.tables.tenant_connections.rows.every((r) => r.members_read_error)).toBe(true);
	});

	it('reads at most a batch per run, the longest-unread first', async () => {
		const rows = Array.from({ length: MEMBER_SYNC_BATCH + 3 }, (_, i) =>
			conn(i + 1, { members_read_at: i < 3 ? null : new Date(NOW - (MEMBER_SYNC_BATCH + 3 - i) * 60_000).toISOString() })
		);
		const d = db(rows);
		const a = adapterFor();
		const out = await runAzureMemberSync(d as never, { cfg: CFG, fetchImpl: entra().fn, adapterFor: a.make, now: NOW });
		expect(out.read).toBe(MEMBER_SYNC_BATCH);
		// Never-read first, then oldest.
		expect(a.read.slice(0, 4)).toEqual(['org-name-1', 'org-name-2', 'org-name-3', 'org-name-4']);
	});

	it('re-proves the grant owner a Project Collection Administrator first — a demoted owner ends nobody (#308 R5b review)', async () => {
		// A demoted owner may still read user entitlements but see only part of the
		// organization: a "complete" read from them would end everyone they cannot see.
		const d = db([conn(1), conn(2)]);
		const a = adapterFor({}, ['org-name-1']);
		const out = await runAzureMemberSync(d as never, { cfg: CFG, fetchImpl: entra().fn, adapterFor: a.make, now: NOW });
		expect(a.proved).toEqual(['org-name-1', 'org-name-2']);
		expect(a.read).toEqual(['org-name-2']);
		expect(out.results[0]).toEqual({ organization: 'org-name-1', error: 'only a Project Collection Administrator of org-name-1 can connect it' });
		expect(d.rpcCalls.filter((c) => c.fn === 'reconcile_unit_members').map((c) => c.args.p_unit)).toEqual(['org-2']);
		expect(d.tables.tenant_connections.rows.find((r) => r.id === 'c-1')?.members_read_error).toMatch(/Project Collection Administrator/);
	});

	it('a read that cannot be recorded never stops the others — and fails the run at the end (#308 R5b review 2)', async () => {
		const d = db([conn(1), conn(2), conn(3)], { message: 'write refused' });
		const a = adapterFor();
		const err = await runAzureMemberSync(d as never, { cfg: CFG, fetchImpl: entra().fn, adapterFor: a.make, now: NOW }).catch((e) => e);
		expect(a.read).toEqual(['org-name-1', 'org-name-2', 'org-name-3']);
		expect(err).toMatchObject({ status: 500 });
		expect(String(err.message)).toMatch(/could not record 3 member reads/);
	});

	it('a failed read of the connections is an error — never "nothing to do"', async () => {
		const d = fakeDojoDb({ tenant_connections: { rows: [], error: { message: 'boom' } } });
		await expect(runAzureMemberSync(d as never, { cfg: CFG, fetchImpl: entra().fn, adapterFor: adapterFor().make, now: NOW })).rejects.toMatchObject({ status: 500 });
	});

	it('a name that now belongs to another organization is recorded, and nobody is ended', async () => {
		const d = db([conn(1, { external_id: 'org-OLD' })]);
		const out = await runAzureMemberSync(d as never, { cfg: CFG, fetchImpl: entra().fn, adapterFor: adapterFor().make, now: NOW });
		expect(out.results[0]).toEqual({ organization: 'org-name-1', error: 'org-name-1 now names a different Azure DevOps organization; nobody was ended' });
		expect(d.rpcCalls.filter((c) => c.fn === 'reconcile_unit_members')).toEqual([]);
	});
});
