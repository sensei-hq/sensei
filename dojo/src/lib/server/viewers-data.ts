// The dōjō's viewers (#307 A3, D8). A viewer is invited into the dōjō (not as a
// member) and an admin then grants them the read-only view of selected workspaces.
// This lists them with their names and granted workspaces, removes one, and grants
// or revokes a workspace. Every write goes through the database's own functions —
// they decide who may (admins of the dōjō) — and a refusal keeps the status the
// database meant. Fails closed: a failed read is an error, never an empty list.
import { AdminError, type DojoClient } from './admin-data';
import { resolveViewerNames } from './identity-resolve';
import { callFn, requireInTenant } from './tenant-rows';
import type { DojoViewerWire as DojoViewer, ViewerWorkspaceView as ViewerWorkspace } from '$lib/viewers-view';

export { AdminError };

// One declaration of the wire row, shared with the Viewers tab that reads it:
// a field renamed on one side fails `bun run check` on the other.
export type { ViewerWorkspaceView as ViewerWorkspace, DojoViewerWire as DojoViewer } from '$lib/viewers-view';

/** The dōjō's viewers, oldest first, each with the dōjō's workspaces granted to them. */
export async function listDojoViewers(db: DojoClient, tenantId: string): Promise<DojoViewer[]> {
	const viewers = await db
		.from('dojo_viewers')
		.select('principal_id, joined_at, email')
		.eq('tenant_id', tenantId)
		.order('joined_at', { ascending: true });
	if (viewers.error) throw new AdminError(500, viewers.error.message);
	const rows = (viewers.data ?? []) as Array<{ principal_id: string; joined_at: string; email: string | null }>;
	if (rows.length === 0) return [];
	const ids = rows.map((r) => r.principal_id);

	const [workspaces, grants, names] = await Promise.all([
		db.from('workspaces').select('id, name').eq('tenant_id', tenantId),
		db.from('workspace_viewers').select('workspace_id, principal_id').in('principal_id', ids),
		resolveViewerNames(db, tenantId, ids)
	]);
	if (workspaces.error) throw new AdminError(500, workspaces.error.message);
	if (grants.error) throw new AdminError(500, grants.error.message);

	const workspaceById = new Map(
		((workspaces.data ?? []) as ViewerWorkspace[]).map((p) => [p.id, { id: p.id, name: p.name }])
	);
	const granted = new Map<string, ViewerWorkspace[]>();
	for (const g of (grants.data ?? []) as Array<{ workspace_id: string; principal_id: string }>) {
		const workspace = workspaceById.get(g.workspace_id);
		if (!workspace) continue; // another dōjō's workspace
		granted.set(g.principal_id, [...(granted.get(g.principal_id) ?? []), workspace]);
	}
	return rows.map((r) => {
		const n = names.get(r.principal_id);
		return {
			principal_id: r.principal_id,
			display_name: n?.display_name ?? null,
			// The forge identity's email, else the one they were invited at.
			email: n?.email ?? r.email ?? null,
			joined_at: r.joined_at,
			workspaces: (granted.get(r.principal_id) ?? []).sort((a, b) => a.name.localeCompare(b.name))
		};
	});
}

/** Remove a viewer from the dōjō, and with it their workspace views here. */
export async function removeDojoViewer(db: DojoClient, tenantId: string, principalId: string): Promise<void> {
	await callFn(db, 'remove_dojo_viewer', { p_tenant: tenantId, p_principal: principalId });
}

/** Grant a person (a member or a dōjō viewer) the view of one of the dōjō's workspaces. */
export async function grantWorkspaceView(
	db: DojoClient,
	tenantId: string,
	workspaceId: string,
	principalId: string
): Promise<void> {
	await requireInTenant(db, 'workspaces', tenantId, workspaceId, 'workspace');
	await callFn(db, 'grant_workspace_viewer', { p_workspace: workspaceId, p_principal: principalId });
}

/** Revoke a person's view of one of the dōjō's workspaces. */
export async function revokeWorkspaceView(
	db: DojoClient,
	tenantId: string,
	workspaceId: string,
	principalId: string
): Promise<void> {
	await requireInTenant(db, 'workspaces', tenantId, workspaceId, 'workspace');
	await callFn(db, 'revoke_workspace_viewer', { p_workspace: workspaceId, p_principal: principalId });
}
