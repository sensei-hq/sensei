// The ELECTION write path.
//
// `sync_enabled = may_share AND elected`. The dōjō computed both from the
// beginning and wrote neither: nothing anywhere inserted into
// `dojo.repository_elections`, so `elected` was false for every repository under
// a USER authority, permanently. The view was right and the answer was always
// no.
//
// ## This module decides nothing
//
// The decision is dojo.set_repository_election's (#236): it reads the CALLER's
// own verdict row of `dojo.all_my_repositories` for the authority and for
// permission, writes what that row says, and re-reads the verdict. It does not
// test `origin`, `visibility`, `role` or billing — that logic lives in the view,
// once, where the daemon and the UI read it too.
//
// The consequence worth stating: **an election cannot be made for a repository
// the caller cannot see**, because the function reads only the caller's rows.
// Scoping is the authorization; there is no second membership check to forget.
import { AdminError, type DojoClient } from './admin-data';
import { refusalStatus } from './db-refusal';

/** What the view says after the write — the verdict, not an echo of the input. */
export interface ElectionOutcome {
	repo_key: string;
	/** The election that was recorded. */
	elected: boolean;
	/** Which authority it was recorded under, per the view. */
	authority: 'user' | 'organization';
	/** `may_share AND elected` — still false when entitlement refuses. */
	sync_enabled: boolean;
	/** Why, when it still will not sync. Null when it will. */
	reason_code: string | null;
}

/**
 * Record an election for one repository, as the signed-in person.
 *
 * Deliberately has NO `authority` parameter. If the caller could name the
 * authority, any member of an organization could write the ORGANIZATION's
 * election and share a repository on everyone else's behalf. The authority is
 * whatever the caller's verdict row says it is.
 *
 * Turning sharing OFF writes `elected = false`; it does not delete the row.
 * `configured_by`/`configured_at` are what let the view distinguish "your
 * organisation turned this off" from "nobody has looked at this yet", and a
 * delete collapses the two back together.
 */
export async function setElection(
	db: DojoClient,
	repoKey: string,
	elected: boolean
): Promise<ElectionOutcome> {
	const { data, error } = await db
		.rpc('set_repository_election', { p_repo_key: repoKey, p_elected: elected })
		.single();
	if (error) throw new AdminError(refusalStatus(error), error.message);
	return data as ElectionOutcome;
}
