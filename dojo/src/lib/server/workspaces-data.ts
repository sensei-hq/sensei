// The workspaces data layer (#299). A workspace is ONE SHARED item — the same as the
// sensei workspace — living in one dōjō (an org's, or a personal dōjō for the
// owner's GitHub account). It has no owner: reads go through RLS
// (dojo.can_read_workspace — whoever may read one of its repositories), the sync
// files it (dojo.file_repository_workspace), and a relay run attaches to it
// (dojo.attach_run_to_workspace). Fails closed — a query error surfaces, never a
// fixture.
import type { dojoDb } from './dojo-supabase';
import { AdminError, type DojoClient } from './admin-data';

export type { DojoClient };
export { AdminError };

/** One workspace row on the wire (mirrors the console `Workspace`). */
export interface WorkspaceRow {
	id: string;
	/** The dōjō the workspace lives in. */
	tenant_id: string;
	slug: string;
	name: string;
	classification: string;
	phase: string;
	last_run_at: string | null;
	runs_week: number;
}

const WORKSPACE_COLS = 'id, tenant_id, slug, name, classification, phase, last_run_at, runs_week';

/**
 * List the tenant's workspaces, most-recently-run first (rows never run sort last
 * by created order). Tenant-scoped; fails closed (AdminError 500).
 */
export async function listOrgWorkspaces(db: DojoClient, tenantId: string): Promise<WorkspaceRow[]> {
	const { data, error } = await db
		.from('workspaces')
		.select(WORKSPACE_COLS)
		.eq('tenant_id', tenantId)
		.order('last_run_at', { ascending: false, nullsFirst: false });
	if (error) throw new AdminError(500, error.message);
	return (data ?? []) as unknown as WorkspaceRow[];
}

/**
 * Every workspace the caller may read, across every dōjō they belong to — the
 * personal `/you/workspaces` list. No owner filter: RLS (dojo.can_read_workspace)
 * is the authorization, so this needs the CALLER's client, never the service
 * role. Most-recently-run first. Fails closed (AdminError 500) — never a
 * fabricated empty list on a query error.
 */
export async function listUserWorkspaces(db: DojoClient): Promise<WorkspaceRow[]> {
	const { data, error } = await db
		.from('workspaces')
		.select(WORKSPACE_COLS)
		.order('last_run_at', { ascending: false, nullsFirst: false });
	if (error) throw new AdminError(500, error.message);
	return (data ?? []) as unknown as WorkspaceRow[];
}

/**
 * The daemon-resolved constitution jsonb for one workspace (the F4 drill-in), by
 * id — a slug is unique only within one dōjō. RLS decides whether the caller may
 * read it: `null` when it has no federated constitution yet OR the caller cannot
 * read it (both → the dōjō shows its "resolves in your editor" state). Fails
 * closed on a query error (AdminError 500) — never a fabricated constitution.
 */
export async function getWorkspaceConstitution(db: DojoClient, workspaceId: string): Promise<unknown | null> {
	const { data, error } = await db
		.from('workspaces')
		.select('constitution')
		.eq('id', workspaceId)
		.maybeSingle();
	if (error) throw new AdminError(500, error.message);
	return (data as { constitution?: unknown } | null)?.constitution ?? null;
}

/**
 * Attach a relay run to the shared workspace it runs in (#299). A run never
 * creates a workspace — the sync does — so this finds (tenant, slug) in the
 * caller's dōjō and records the run time and, when the daemon resolved one, the
 * constitution. `null` when the dōjō has no such workspace yet. Throws AdminError
 * on an error; the route keeps it off the federation path.
 */
export async function attachRunToWorkspace(
	db: DojoClient,
	tenantId: string,
	slug: string,
	constitution: unknown
): Promise<string | null> {
	const { data, error } = await db.rpc('attach_run_to_workspace', {
		p_tenant_id: tenantId,
		p_slug: slug,
		p_constitution: constitution ?? null
	});
	if (error) throw new AdminError(500, error.message);
	return (data as string | null) ?? null;
}

/**
 * File a relay session under its shared workspace (#252, #299) — what lets a team
 * that owns the workspace see THAT the run is going (`dojo.peer_run_status`,
 * status only). The session policy accepts only a workspace in a dōjō the caller
 * is a member of. Throws
 * AdminError on a DB error; the route keeps it off the federation path.
 */
export async function fileRunUnderWorkspace(db: DojoClient, sessionId: string, workspaceId: string): Promise<void> {
	const { error } = await db.from('relay_sessions').update({ workspace_id: workspaceId }).eq('id', sessionId);
	if (error) throw new AdminError(500, error.message);
}
