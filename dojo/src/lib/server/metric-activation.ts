// The ACTIVATION write path: a tenant switching a catalogue metric off for one
// repository.
//
// ## Absence means enabled, so enabling DELETES
//
// The whole feature rests on that invariant — a metric added to the catalogue
// later must be on everywhere without anyone touching a row. Storing
// `enabled = true` would make this table authoritative for "on" as well as
// "off", and then tomorrow's metric would arrive OFF for every repository no row
// happens to mention. So the column exists (the DDL declares it, and a future
// tri-state may want it) but this path only ever writes `false` or removes.
//
// ## Authorization is the view, as it is for elections
//
// `dojo.all_my_repositories` is per-principal and already carries
// `configurable_by_me`. Reading the verdict from there rather than re-deriving
// role logic means there is no second membership check to forget, and no fourth
// copy of the rule the view exists to own.
//
// `configurable_by_me` is admin-only, which is the right bar: deactivation stops
// a computation for everyone sharing the repository and is a cost decision, not a
// per-member display preference.
import { AdminError, type DojoClient } from './admin-data';

/** What the tenant's ruling now is, re-read rather than echoed from the input. */
export interface ActivationOutcome {
	repo_key: string;
	metric: string;
	/** `true` = no row (the default). `false` = a stored deactivation. */
	enabled: boolean;
	tenant: string;
}

interface ViewRow {
	repository_id: string;
	tenant_id: string;
	tenant: string;
	repo_key: string;
	configurable_by_me: boolean;
}

/** Named rather than `*` so a view column rename fails here instead of silently
 *  reading `undefined` as "not allowed". */
const VIEW_COLUMNS = 'repository_id, tenant_id, tenant, repo_key, configurable_by_me';

async function readMine(
	db: DojoClient,
	principalId: string,
	repoKey: string
): Promise<ViewRow | null> {
	const { data, error } = await db
		.from('all_my_repositories')
		.select(VIEW_COLUMNS)
		.eq('principal_id', principalId)
		.eq('repo_key', repoKey)
		.maybeSingle();
	// Never degrade a failed read to "not found": a 404 tells the caller the
	// repository is not theirs, which is a different and load-bearing claim.
	if (error) throw new AdminError(500, error.message);
	return (data ?? null) as ViewRow | null;
}

/**
 * Switch one catalogue metric on or off for one repository under one tenant.
 *
 * `metricKey`, not an id: `sensei.metrics.id` differs between the daemon and dōjō
 * planes (separate databases loaded from the same staging file), so the key is
 * the only value both speak. Resolved through `dojo.metric_catalogue`, the view
 * over `sensei.metrics` — the `sensei` schema is deliberately not exposed to
 * PostgREST, so a bare `.from('metrics')` answers "Could not find the table
 * 'dojo.metrics'".
 */
export async function setMetricActivation(
	db: DojoClient,
	principalId: string,
	repoKey: string,
	metricKey: string,
	enabled: boolean
): Promise<ActivationOutcome> {
	const mine = await readMine(db, principalId, repoKey);
	if (!mine) throw new AdminError(404, `no repository ${repoKey} for this account`);
	if (!mine.configurable_by_me) {
		throw new AdminError(403, `you may not change metric activation for ${repoKey}`);
	}

	const { data: mData, error: mErr } = await db
		.from('metric_catalogue')
		.select('id')
		.eq('key', metricKey)
		.maybeSingle();
	if (mErr) throw new AdminError(500, `metric_catalogue: ${mErr.message}`);
	const metric = mData as { id: string } | null;
	// An unknown key would store a deactivation that matches nothing, forever,
	// and could not be undone through this API because the same lookup would miss.
	if (!metric) throw new AdminError(404, `unknown metric ${metricKey}`);

	if (enabled) {
		// Absence IS enabled. A delete that matches nothing is the correct no-op:
		// the UI may send the current state back, and refusing would make a
		// harmless double-click read as a failure.
		const { error } = await db
			.from('metric_activations')
			.delete()
			.eq('tenant_id', mine.tenant_id)
			.eq('repository_id', mine.repository_id)
			.eq('metric_id', metric.id);
		if (error) throw new AdminError(500, error.message);
		return { repo_key: mine.repo_key, metric: metricKey, enabled: true, tenant: mine.tenant };
	}

	// Read-then-write rather than upsert: the row is addressed by a three-column
	// key and PostgREST's `on_conflict` takes a constraint BY NAME, which couples
	// this file to a DDL identifier for no gain at one row per toggle.
	const existing = await db
		.from('metric_activations')
		.select('tenant_id')
		.eq('tenant_id', mine.tenant_id)
		.eq('repository_id', mine.repository_id)
		.eq('metric_id', metric.id);
	if (existing.error) throw new AdminError(500, existing.error.message);
	if ((existing.data ?? []).length > 0) {
		// Already off. Refresh nothing else — `updated_at` has a default and the
		// value is unchanged, so a write here would be noise.
		return { repo_key: mine.repo_key, metric: metricKey, enabled: false, tenant: mine.tenant };
	}

	const { error } = await db.from('metric_activations').insert({
		tenant_id: mine.tenant_id,
		repository_id: mine.repository_id,
		metric_id: metric.id,
		enabled: false
	});
	if (error) throw new AdminError(500, error.message);
	return { repo_key: mine.repo_key, metric: metricKey, enabled: false, tenant: mine.tenant };
}
