// The dōjō's viewers (#307 A3, D8). A viewer is invited into the dōjō (not as a
// member) and an admin then grants them the read-only view of selected projects.
// This lists them with their names and granted projects, removes one, and grants
// or revokes a project. Every write goes through the database's own functions —
// they decide who may (admins of the dōjō) — and a refusal keeps the status the
// database meant. Fails closed: a failed read is an error, never an empty list.
import { AdminError, type DojoClient } from './admin-data';
import { resolveViewerNames } from './identity-resolve';
import { callFn, requireInTenant } from './tenant-rows';

export { AdminError };

export interface ViewerProject {
	id: string;
	name: string;
}

export interface DojoViewer {
	principal_id: string;
	display_name: string | null;
	email: string | null;
	joined_at: string;
	projects: ViewerProject[];
}

/** The dōjō's viewers, oldest first, each with the dōjō's projects granted to them. */
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

	const [projects, grants, names] = await Promise.all([
		db.from('projects').select('id, name').eq('tenant_id', tenantId),
		db.from('project_viewers').select('project_id, principal_id').in('principal_id', ids),
		resolveViewerNames(db, tenantId, ids)
	]);
	if (projects.error) throw new AdminError(500, projects.error.message);
	if (grants.error) throw new AdminError(500, grants.error.message);

	const projectById = new Map(
		((projects.data ?? []) as ViewerProject[]).map((p) => [p.id, { id: p.id, name: p.name }])
	);
	const granted = new Map<string, ViewerProject[]>();
	for (const g of (grants.data ?? []) as Array<{ project_id: string; principal_id: string }>) {
		const project = projectById.get(g.project_id);
		if (!project) continue; // another dōjō's project
		granted.set(g.principal_id, [...(granted.get(g.principal_id) ?? []), project]);
	}
	return rows.map((r) => {
		const n = names.get(r.principal_id);
		return {
			principal_id: r.principal_id,
			display_name: n?.display_name ?? null,
			// The forge identity's email, else the one they were invited at.
			email: n?.email ?? r.email ?? null,
			joined_at: r.joined_at,
			projects: (granted.get(r.principal_id) ?? []).sort((a, b) => a.name.localeCompare(b.name))
		};
	});
}

/** Remove a viewer from the dōjō, and with it their project views here. */
export async function removeDojoViewer(db: DojoClient, tenantId: string, principalId: string): Promise<void> {
	await callFn(db, 'remove_dojo_viewer', { p_tenant: tenantId, p_principal: principalId });
}

/** Grant a person (a member or a dōjō viewer) the view of one of the dōjō's projects. */
export async function grantProjectView(
	db: DojoClient,
	tenantId: string,
	projectId: string,
	principalId: string
): Promise<void> {
	await requireInTenant(db, 'projects', tenantId, projectId, 'project');
	await callFn(db, 'grant_project_viewer', { p_project: projectId, p_principal: principalId });
}

/** Revoke a person's view of one of the dōjō's projects. */
export async function revokeProjectView(
	db: DojoClient,
	tenantId: string,
	projectId: string,
	principalId: string
): Promise<void> {
	await requireInTenant(db, 'projects', tenantId, projectId, 'project');
	await callFn(db, 'revoke_project_viewer', { p_project: projectId, p_principal: principalId });
}
