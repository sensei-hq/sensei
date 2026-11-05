// The workspaces data layer (#299). A workspace is ONE SHARED item — the same as the
// sensei workspace — living in one dōjō (an org's, or a personal dōjō for the
// owner's GitHub account). It has no owner: reads go through RLS
// (dojo.can_read_workspace — whoever may read one of its repositories), the sync
// files it (dojo.file_repository_workspace), and a relay run attaches to it
// (dojo.attach_run_to_workspace). Fails closed — a query error surfaces, never a
// fixture.
import type { dojoDb } from './dojo-supabase';
import { AdminError, type DojoClient } from './admin-data';
import { readAllPages } from './repositories';
import { callFn, requireInTenant } from './tenant-rows';
import { num, windowSince, type Num } from './team-metrics-data';

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
 * Order `rows` most-recently-active first by `dojo.workspace_activity`
 * (latest reading landing or graph sync per workspace, computed in the DB).
 * One read narrowed to the listed ids (a whole-view read would hit max_rows
 * and truncate with no error); quiet workspaces keep their API order, last —
 * the sort is stable and attaches nothing to the rows, so no date is invented.
 * Fails closed (AdminError 500): a broken activity read is an error, never the
 * unsorted list. No rows, no read.
 */
async function sortByActivity(db: DojoClient, rows: WorkspaceRow[]): Promise<WorkspaceRow[]> {
	if (rows.length === 0) return rows;
	const { data, error } = await db
		.from('workspace_activity')
		.select('workspace_id, last_activity_at')
		.in(
			'workspace_id',
			rows.map((w) => w.id)
		);
	if (error) throw new AdminError(500, error.message);
	const at = new Map<string, number>();
	for (const r of (data ?? []) as unknown as Array<{ workspace_id: string; last_activity_at: string | null }>) {
		const t = r.last_activity_at ? Date.parse(r.last_activity_at) : NaN;
		if (!Number.isNaN(t)) at.set(r.workspace_id, t);
	}
	return [...rows].sort((a, b) => {
		const ta = at.get(a.id);
		const tb = at.get(b.id);
		if (ta === undefined && tb === undefined) return 0;
		if (ta === undefined) return 1;
		if (tb === undefined) return -1;
		return tb - ta;
	});
}

/**
 * List the tenant's workspaces, most-recently-active first (a workspace whose
 * repositories pushed nothing sorts last, by API order). Tenant-scoped; fails
 * closed (AdminError 500).
 */
export async function listOrgWorkspaces(db: DojoClient, tenantId: string): Promise<WorkspaceRow[]> {
	const { data, error } = await db
		.from('workspaces')
		.select(WORKSPACE_COLS)
		.eq('tenant_id', tenantId)
		.order('last_run_at', { ascending: false, nullsFirst: false });
	if (error) throw new AdminError(500, error.message);
	return sortByActivity(db, (data ?? []) as unknown as WorkspaceRow[]);
}

/**
 * Every workspace the caller may read, across every dōjō they belong to — the
 * personal `/you/workspaces` list. No owner filter: RLS (dojo.can_read_workspace)
 * is the authorization, so this needs the CALLER's client, never the service
 * role. Most-recently-active first. Fails closed (AdminError 500) — never a
 * fabricated empty list on a query error.
 */
export async function listUserWorkspaces(db: DojoClient): Promise<WorkspaceRow[]> {
	const { data, error } = await db
		.from('workspaces')
		.select(WORKSPACE_COLS)
		.order('last_run_at', { ascending: false, nullsFirst: false });
	if (error) throw new AdminError(500, error.message);
	return sortByActivity(db, (data ?? []) as unknown as WorkspaceRow[]);
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

/** One series row as `dojo.workspace_metric_series` returns it. */
interface WorkspaceSeriesRow {
	repository_id: string;
	repo_key: string;
	metric_key: string;
	scope: string;
	principal_id: string | null;
	computed_on: string;
	grain: string;
	value: Num;
	props: unknown;
}

/** One series row on the wire. `principal_id` is null on repo rows AND on the
 *  viewer-pooled rows (scope `pooled`, never `user`) — the screen must not
 *  treat "no principal" as "no people behind it" (the pool's
 *  `props.pooled_rows` says how many), and must never sum `pooled` rows
 *  together with the `user` detail rows of the same day. */
export interface WorkspaceMetricRow {
	repository_id: string;
	repo_key: string;
	metric_key: string;
	scope: string;
	principal_id: string | null;
	day: string;
	value: number;
	props: unknown;
}

/** The workspace series on the wire: flat rows in a deterministic order — by
 *  repository, metric, scope, day, then principal (pooled rows, with no
 *  principal, first) — so paging never drops or repeats a row. The
 *  per-repository split is the screen's shaping (WP5), not this read's. */
export interface WorkspaceMetricsWire {
	since: string;
	days: number;
	rows: WorkspaceMetricRow[];
}

/**
 * A workspace's metric series over the last `days` ending `today` (WP4).
 *
 * THREE answers, decided in order: the gate (403 — whoever may read the
 * workspace: one of its repositories, a viewer grant, a dōjō admin), the tenant
 * scope (404 — the addressed dōjō's, never reached through another's URL),
 * then the rollup. `db` is the caller's client throughout, so the rollup's
 * definer branches still answer only for them.
 *
 * Fails closed: a series error is a 500, never [].
 */
export async function getWorkspaceMetrics(
	db: DojoClient,
	tenantId: string,
	workspaceId: string,
	opts: { days: number; today: Date }
): Promise<WorkspaceMetricsWire> {
	const mayRead = await callFn<boolean>(db, 'can_read_workspace', { p_workspace: workspaceId });
	if (!mayRead) throw new AdminError(403, 'you may not read this workspace’s metrics');
	await requireInTenant(db, 'workspaces', tenantId, workspaceId, 'workspace');
	const since = windowSince(opts.today, opts.days);
	// A deterministic total order: each page re-runs the rollup with a fresh
	// OFFSET, and without one a row past 1,000 can land on two pages or on
	// none. Unique per row — within one branch the table's natural key is
	// unique, and a pooled row carries a null principal where a detail row
	// carries a real one.
	const rows = await readAllPages<WorkspaceSeriesRow>((from, to) =>
		db
			.rpc('workspace_metric_series', { p_workspace_id: workspaceId, p_since: since })
			.select('repository_id, repo_key, metric_key, scope, principal_id, computed_on, grain, value, props')
			.order('repository_id')
			.order('metric_key')
			.order('scope')
			.order('computed_on')
			.order('principal_id', { nullsFirst: true })
			.range(from, to)
	);
	return {
		since,
		days: opts.days,
		rows: rows.map((r) => ({
			repository_id: r.repository_id,
			repo_key: r.repo_key,
			metric_key: r.metric_key,
			scope: r.scope,
			principal_id: r.principal_id,
			day: r.computed_on,
			value: num(r.value),
			props: r.props
		}))
	};
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
