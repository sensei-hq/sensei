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
 * List ALL the caller's projects across every dōjō (user-primary) — their
 * `dojo.projects` rows regardless of tenant, most-recently-run first. Backs the
 * personal `/you/projects` list. The `user_id` filter IS the authorization (a
 * caller only ever sees their own rows), mirroring `listUserOrgs`; the row's
 * `user_id` is set by `upsertProjectFromRun` from the authenticated caller,
 * never the payload. Fails closed (AdminError 500) — never a fabricated empty
 * list on a query error.
 */
export async function listUserProjects(db: DojoClient, userId: string): Promise<ProjectRow[]> {
	const { data, error } = await db
		.from('projects')
		.select(PROJECT_COLS)
		.eq('user_id', userId)
		.order('last_run_at', { ascending: false, nullsFirst: false });
	if (error) throw new AdminError(500, error.message);
	return (data ?? []) as unknown as ProjectRow[];
}

/**
 * The daemon-resolved constitution jsonb for one of the caller's projects (the
 * F4 drill-in read), scoped by `user_id` + `slug` (the filter IS the authz).
 * Returns the stored `RelayConstitution` shape, or `null` when the project has no
 * federated constitution yet OR there is no such row for this user (both → the
 * dōjō shows its honest "resolves in your editor" state). Fails closed on a query
 * error (AdminError 500) — never a fabricated constitution.
 */
export async function getUserProjectConstitution(
	db: DojoClient,
	userId: string,
	slug: string
): Promise<unknown | null> {
	const { data, error } = await db
		.from('projects')
		.select('constitution')
		.eq('user_id', userId)
		.eq('slug', slug)
		.maybeSingle();
	if (error) throw new AdminError(500, error.message);
	return (data as { constitution?: unknown } | null)?.constitution ?? null;
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

/** The run's project as the daemon federates it (`RelayProjectInfo`). */
export interface RunProjectInput {
	slug: string;
	name: string;
	classification: string;
	phase: string;
	/** The daemon-resolved constitution (the `RelayConstitution` jsonb) for the
	 *  drill-in preview (F4). Written when present; OMITTED when absent so an
	 *  existing resolution is preserved on conflict (like `phase`), never nulled
	 *  by a heartbeat that didn't carry one. */
	constitution?: unknown;
}

/**
 * Upsert the run's project into `dojo.projects` (the daemon's relay/session POST
 * carries it) — the write side of the projects screens. The OWNER is the
 * authenticated caller (`user_id`/`tenant_id` are never taken from the payload); a
 * `personal` project is tenant-less so an unbound run fanned out to several dōjōs
 * still resolves to one `unique(user_id, slug)` row. `phase` is intentionally NOT
 * written: on insert the column default (`watch`) applies, and on conflict the
 * existing phase is left untouched — a later dōjō-side advance (watch→notice→adopt)
 * is never clobbered by a run heartbeat. Throws AdminError(500) on a DB error; the
 * caller runs it fire-and-forget so a failure never breaks relay federation.
 */
export async function upsertProjectFromRun(
	db: DojoClient,
	caller: { userId: string; tenantId: string },
	p: RunProjectInput
): Promise<void> {
	const nowIso = new Date().toISOString();
	const row: Record<string, unknown> = {
		user_id: caller.userId,
		tenant_id: p.classification === 'personal' ? null : caller.tenantId,
		slug: p.slug,
		name: p.name,
		classification: p.classification,
		last_run_at: nowIso,
		updated_at: nowIso
	};
	// Write the constitution only when the daemon federated one — omitting the key
	// leaves any existing resolution untouched on conflict (never nulled), the same
	// preserve-on-conflict discipline as `phase`.
	if (p.constitution !== undefined && p.constitution !== null) {
		row.constitution = p.constitution;
	}
	const { error } = await db.from('projects').upsert(row, { onConflict: 'user_id,slug' });
	if (error) throw new AdminError(500, error.message);
}
