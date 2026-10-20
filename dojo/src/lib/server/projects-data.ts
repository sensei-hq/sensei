// The projects data layer (#299). A project is ONE SHARED item — the same as the
// sensei project — living in one dōjō (an org's, or a personal dōjō for the
// owner's GitHub account). It has no owner: reads go through RLS
// (dojo.can_read_project — whoever may read one of its repositories), the sync
// files it (dojo.file_repository_project), and a relay run attaches to it
// (dojo.attach_run_to_project). Fails closed — a query error surfaces, never a
// fixture.
import type { dojoDb } from './dojo-supabase';
import { AdminError, type DojoClient } from './admin-data';

export type { DojoClient };
export { AdminError };

/** One project row on the wire (mirrors the console `Project`). */
export interface ProjectRow {
	id: string;
	/** The dōjō the project lives in. */
	tenant_id: string;
	slug: string;
	name: string;
	classification: string;
	phase: string;
	last_run_at: string | null;
	runs_week: number;
}

const PROJECT_COLS = 'id, tenant_id, slug, name, classification, phase, last_run_at, runs_week';

/**
 * List the tenant's projects, most-recently-run first (rows never run sort last
 * by created order). Tenant-scoped; fails closed (AdminError 500).
 */
export async function listOrgProjects(db: DojoClient, tenantId: string): Promise<ProjectRow[]> {
	const { data, error } = await db
		.from('projects')
		.select(PROJECT_COLS)
		.eq('tenant_id', tenantId)
		.order('last_run_at', { ascending: false, nullsFirst: false });
	if (error) throw new AdminError(500, error.message);
	return (data ?? []) as unknown as ProjectRow[];
}

/**
 * Every project the caller may read, across every dōjō they belong to — the
 * personal `/you/projects` list. No owner filter: RLS (dojo.can_read_project)
 * is the authorization, so this needs the CALLER's client, never the service
 * role. Most-recently-run first. Fails closed (AdminError 500) — never a
 * fabricated empty list on a query error.
 */
export async function listUserProjects(db: DojoClient): Promise<ProjectRow[]> {
	const { data, error } = await db
		.from('projects')
		.select(PROJECT_COLS)
		.order('last_run_at', { ascending: false, nullsFirst: false });
	if (error) throw new AdminError(500, error.message);
	return (data ?? []) as unknown as ProjectRow[];
}

/**
 * The daemon-resolved constitution jsonb for one project (the F4 drill-in), by
 * id — a slug is unique only within one dōjō. RLS decides whether the caller may
 * read it: `null` when it has no federated constitution yet OR the caller cannot
 * read it (both → the dōjō shows its "resolves in your editor" state). Fails
 * closed on a query error (AdminError 500) — never a fabricated constitution.
 */
export async function getProjectConstitution(db: DojoClient, projectId: string): Promise<unknown | null> {
	const { data, error } = await db
		.from('projects')
		.select('constitution')
		.eq('id', projectId)
		.maybeSingle();
	if (error) throw new AdminError(500, error.message);
	return (data as { constitution?: unknown } | null)?.constitution ?? null;
}

/**
 * Attach a relay run to the shared project it runs in (#299). A run never
 * creates a project — the sync does — so this finds (tenant, slug) in the
 * caller's dōjō and records the run time and, when the daemon resolved one, the
 * constitution. `null` when the dōjō has no such project yet. Throws AdminError
 * on an error; the route keeps it off the federation path.
 */
export async function attachRunToProject(
	db: DojoClient,
	tenantId: string,
	slug: string,
	constitution: unknown
): Promise<string | null> {
	const { data, error } = await db.rpc('attach_run_to_project', {
		p_tenant_id: tenantId,
		p_slug: slug,
		p_constitution: constitution ?? null
	});
	if (error) throw new AdminError(500, error.message);
	return (data as string | null) ?? null;
}

/**
 * File a relay session under its shared project (#252, #299) — what lets a team
 * that owns the project see THAT the run is going (`dojo.peer_run_status`,
 * status only). The session policy accepts only a project in a dōjō the caller
 * is a member of. Throws
 * AdminError on a DB error; the route keeps it off the federation path.
 */
export async function fileRunUnderProject(db: DojoClient, sessionId: string, projectId: string): Promise<void> {
	const { error } = await db.from('relay_sessions').update({ project_id: projectId }).eq('id', sessionId);
	if (error) throw new AdminError(500, error.message);
}
