// The dōjō's viewers (#307 A3, D8): people invited in as viewers — not members —
// and the workspaces an admin has granted each. The admin lists them, removes one,
// and grants or revokes a workspace. Refusals keep the status the database meant.
import { describe, it, expect } from 'vitest';
import { fakeDojoDb, type FakeTable } from './fake-dojo-db';
import { listDojoViewers, removeDojoViewer, grantWorkspaceView, revokeWorkspaceView } from './viewers-data';
import { AdminError, type DojoClient } from './admin-data';

const T = 'tenant-t';

/** The fake, typed as the client the module takes (it records `rpcCalls` too). */
function db(over: Record<string, FakeTable> = {}) {
	return asClient(fakeDojoDb({
		dojo_viewers: {
			rows: [
				{ tenant_id: T, principal_id: 'boss', joined_at: '2026-10-20T00:00:00Z' },
				{ tenant_id: T, principal_id: 'client', joined_at: '2026-10-21T00:00:00Z', email: 'client@example.com' },
				{ tenant_id: 'other', principal_id: 'elsewhere', joined_at: '2026-10-21T00:00:00Z' }
			]
		},
		workspaces: {
			rows: [
				{ id: 'p1', tenant_id: T, name: 'Payments' },
				{ id: 'p2', tenant_id: T, name: 'Search' },
				{ id: 'px', tenant_id: 'other', name: 'Not ours' }
			]
		},
		workspace_viewers: {
			rows: [
				{ workspace_id: 'p1', principal_id: 'boss' },
				{ workspace_id: 'p2', principal_id: 'boss' },
				{ workspace_id: 'px', principal_id: 'boss' },
				{ workspace_id: 'p1', principal_id: 'a-member' }
			]
		},
		identities: {
			rows: [
				{ principal_id: 'boss', display_name: 'The Boss', email: 'boss@example.com', last_login_at: null },
				{ principal_id: 'elsewhere', display_name: 'Not here', email: 'x@example.com', last_login_at: null }
			]
		},
		'rpc:remove_dojo_viewer': { rows: [] },
		'rpc:grant_workspace_viewer': { rows: [] },
		'rpc:revoke_workspace_viewer': { rows: [] },
		...over
	}));
}
const asClient = (f: ReturnType<typeof fakeDojoDb>) => f as unknown as DojoClient & typeof f;

describe('listDojoViewers', () => {
	it("lists the dōjō's viewers with their names and the dōjō's workspaces granted to each", async () => {
		const got = await listDojoViewers(db(), T);
		expect(got).toEqual([
			{
				principal_id: 'boss',
				display_name: 'The Boss',
				email: 'boss@example.com',
				joined_at: '2026-10-20T00:00:00Z',
				workspaces: [
					{ id: 'p1', name: 'Payments' },
					{ id: 'p2', name: 'Search' }
				]
			},
			// No identity: named by the email they were invited at.
			{ principal_id: 'client', display_name: null, email: 'client@example.com', joined_at: '2026-10-21T00:00:00Z', workspaces: [] }
		]);
	});

	it('a failed read is an error, never an empty list', async () => {
		await expect(listDojoViewers(db({ dojo_viewers: { rows: [], error: { message: 'boom' } } }), T)).rejects.toThrow(
			AdminError
		);
		await expect(listDojoViewers(db({ workspace_viewers: { rows: [], error: { message: 'boom' } } }), T)).rejects.toThrow(
			AdminError
		);
	});
});

describe('viewer writes', () => {
	it('remove asks the database to remove the viewer from this dōjō', async () => {
		const d = db();
		await removeDojoViewer(d, T, 'boss');
		expect(d.rpcCalls).toEqual([{ fn: 'remove_dojo_viewer', args: { p_tenant: T, p_principal: 'boss' } }]);
	});

	it("grant and revoke name the workspace and the person, for this dōjō's workspaces only", async () => {
		const d = db();
		await grantWorkspaceView(d, T, 'p2', 'client');
		await revokeWorkspaceView(d, T, 'p1', 'boss');
		expect(d.rpcCalls).toEqual([
			{ fn: 'grant_workspace_viewer', args: { p_workspace: 'p2', p_principal: 'client' } },
			{ fn: 'revoke_workspace_viewer', args: { p_workspace: 'p1', p_principal: 'boss' } }
		]);
		await expect(grantWorkspaceView(d, T, 'px', 'client')).rejects.toMatchObject({ status: 404 });
	});

	it('keeps the status the database meant', async () => {
		const refused = (code: string) => db({ 'rpc:grant_workspace_viewer': { rows: [], error: { code, message: 'no' } } });
		await expect(grantWorkspaceView(refused('42501'), T, 'p1', 'x')).rejects.toMatchObject({ status: 403 });
		// Not in the dōjō: invite them as a viewer first.
		await expect(grantWorkspaceView(refused('23503'), T, 'p1', 'x')).rejects.toMatchObject({ status: 409 });
	});
});
