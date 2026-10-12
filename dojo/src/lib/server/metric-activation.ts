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
// ## Authorization is the database's, as it is for elections
//
// dojo.set_metric_activation reads the CALLER's own verdict row of
// `dojo.all_my_repositories` (#236): there is no second membership check to
// forget, and no fourth copy of the rule the view exists to own.
//
// The view's `configurable_by_me` is NOT enough on its own. It is true for any
// member on a personal repository and on an organisation's PUBLIC repository —
// right for the per-user share election it was built for. But an activation row
// has no principal: it stops a computation for EVERY member of the tenant, a
// cost decision that is the organisation's policy. So on an organisation
// repository it also takes an admin; in a personal dōjō the tenant is the person
// (docs/spec/dojo/access-matrix.md "Metrics"). The function holds both.
import { AdminError, type DojoClient } from './admin-data';
import { refusalStatus } from './db-refusal';

/** What the tenant's ruling now is, re-read rather than echoed from the input. */
export interface ActivationOutcome {
	repo_key: string;
	metric: string;
	/** `true` = no row (the default). `false` = a stored deactivation. */
	enabled: boolean;
	tenant: string;
}

/**
 * Switch one catalogue metric off (or back on) for one of the caller's
 * repositories. Off is a stored row; on is its absence — enabling deletes.
 */
export async function setMetricActivation(
	db: DojoClient,
	repoKey: string,
	metricKey: string,
	enabled: boolean
): Promise<ActivationOutcome> {
	const { data, error } = await db
		.rpc('set_metric_activation', { p_repo_key: repoKey, p_metric_key: metricKey, p_enabled: enabled })
		.single();
	if (error) throw new AdminError(refusalStatus(error), error.message);
	return data as ActivationOutcome;
}
