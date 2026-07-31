// Console read for the org Projects screen (`GET …/projects`) — the tenant's
// `dojo.projects` rows (the daemon upserts them on relay runs). Replaces the
// `orgProjectsFor(slug)` fixture that rendered fabricated repos for real orgs
// (F4 honesty): a real read, honest-empty until the daemon populates the table.
// Fails closed — a query error surfaces, never a fixture.
import type { dojoDb } from './dojo-supabase';
import { AdminError, type DojoClient } from './admin-data';

export type { DojoClient };
export { AdminError };

/** One project row on the wire (mirrors the console `Project`). */
export interface ProjectRow {
	id: string;
	slug: string;
	name: string;
	classification: string;
	phase: string;
	last_run_at: string | null;
	runs_week: number;
}

const PROJECT_COLS = 'id, slug, name, classification, phase, last_run_at, runs_week';

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
 * Count the tenant's projects (for the org-home / my-dojos project count). One
 * head/count query; fails closed.
 */
export async function countOrgProjects(db: DojoClient, tenantId: string): Promise<number> {
	const { count, error } = await db
		.from('projects')
		.select('id', { count: 'exact', head: true })
		.eq('tenant_id', tenantId);
	if (error) throw new AdminError(500, error.message);
	return count ?? 0;
}
