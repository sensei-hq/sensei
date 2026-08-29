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
// It reads `dojo.all_my_repositories` for the authority and for permission, and
// writes what that row says. It does not test `origin`, `visibility`, `role` or
// billing — that logic lives in the view, once, where the daemon and the UI read
// it too. Re-deriving it here would be the fourth copy the view exists to
// remove, and the copies drift silently: the one that refuses is invisible until
// someone reports that a toggle did nothing.
//
// The consequence worth stating: **an election cannot be made for a repository
// the caller cannot see**, because the view is already per-principal. Scoping is
// the authorization; there is no second membership check to forget.
import { AdminError, type DojoClient } from './admin-data';

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

/** The columns the write path needs. Named rather than `*` so a view column
 *  rename fails here instead of silently reading `undefined` as "not allowed". */
const VIEW_COLUMNS =
	'repository_id, tenant_id, repo_key, authority, configurable_by_me, elected, sync_enabled, reason_code, reason, reason_actor';

interface ViewRow {
	repository_id: string;
	tenant_id: string;
	repo_key: string;
	authority: 'user' | 'organization' | null;
	configurable_by_me: boolean;
	elected: boolean;
	sync_enabled: boolean;
	reason_code: string | null;
	reason: string | null;
	reason_actor: string | null;
}

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
 * Record an election for one repository, on behalf of one principal.
 *
 * Deliberately has NO `authority` parameter. If the caller could name the
 * authority, any member of an organization could write the ORGANIZATION's
 * election and share a repository on everyone else's behalf. The authority is
 * whatever the view says it is.
 *
 * Turning sharing OFF writes `elected = false`; it does not delete the row.
 * `configured_by`/`configured_at` are what let the view distinguish "your
 * organisation turned this off" from "nobody has looked at this yet", and a
 * delete collapses the two back together.
 */
export async function setElection(
	db: DojoClient,
	principalId: string,
	repoKey: string,
	elected: boolean
): Promise<ElectionOutcome> {
	const mine = await readMine(db, principalId, repoKey);
	if (!mine) throw new AdminError(404, `no repository ${repoKey} for this account`);

	// No captured visibility means no derivable authority — nobody holds the
	// choice yet. Accepting the election would file it under a GUESSED authority
	// that the next capture can contradict, leaving a row the view no longer
	// reads: a decision that was made and silently stopped counting.
	if (mine.authority === null) {
		throw new AdminError(
			409,
			`${repoKey} has no authority yet: ${mine.reason ?? mine.reason_code ?? 'forge visibility is not captured'}`
		);
	}

	// Permission, as the view computed it. The message carries the view's own
	// reason and actor so the refusal names who CAN act, rather than only that
	// the caller cannot.
	if (!mine.configurable_by_me) {
		const who = mine.reason_actor ? ` (${mine.reason_actor} can change it)` : '';
		throw new AdminError(403, `you may not change sharing for ${repoKey}${who}`);
	}

	// An org election is the tenant's, so it carries NO principal — the CHECK
	// constraint requires that, and the view joins the org slot on it.
	const principalSlot = mine.authority === 'user' ? principalId : null;

	await writeElection(db, mine, mine.authority, principalSlot, elected);

	// Re-read rather than report success from the input. Electing is half the
	// decision: a repository whose entitlement still refuses must come back
	// `sync_enabled: false`, or the UI claims it is shared and the daemon then
	// declines to push it.
	const after = await readMine(db, principalId, repoKey);
	if (!after) throw new AdminError(500, `election for ${repoKey} vanished on re-read`);
	return {
		repo_key: after.repo_key,
		elected: after.elected,
		authority: mine.authority,
		sync_enabled: after.sync_enabled,
		reason_code: after.reason_code
	};
}

/**
 * Insert-or-update the one row for (repository, authority, principal).
 *
 * Written as read-then-write rather than an upsert because the row is addressed
 * by a three-column key with a NULLABLE member, and PostgREST's `on_conflict`
 * takes a constraint by name — which couples this file to a DDL identifier for
 * no gain at this volume (one row per toggle).
 *
 * The insert races: two toggles at once can both miss the read. That is what the
 * unique is for, so a duplicate-key error is retried as an update rather than
 * surfaced — the second writer's value wins, which is what "last write wins" on
 * a toggle already means. Any OTHER error is propagated.
 */
async function writeElection(
	db: DojoClient,
	mine: ViewRow,
	authority: 'user' | 'organization',
	principalSlot: string | null,
	elected: boolean
): Promise<void> {
	const repositoryId = mine.repository_id;
	// `elected_at` is set explicitly on UPDATE and left to the column default on
	// INSERT. There is no `elected_by`: WHO decided is already implied by the
	// authority — an org election is the tenant's and carries no principal by
	// CHECK constraint, so a `principal_id`-shaped "who" could only be recorded
	// for half the rows, and the view reports `configured_by` as the AUTHORITY.
	const stamp = () => ({ elected, elected_at: new Date().toISOString() });
	// `.eq(col, null)` renders `col=eq.null` and matches NOTHING. The org slot's
	// principal IS null, so using `.eq` here would miss the existing row on every
	// org toggle and insert a duplicate — caught by the unique, then retried as an
	// update that also misses. `.is` is the only correct spelling for null.
	const existing = await (principalSlot === null
		? db
				.from('repository_elections')
				.select('id')
				.eq('repository_id', repositoryId)
				.eq('authority', authority)
				.is('principal_id', null)
				.maybeSingle()
		: db
				.from('repository_elections')
				.select('id')
				.eq('repository_id', repositoryId)
				.eq('authority', authority)
				.eq('principal_id', principalSlot)
				.maybeSingle());
	if (existing.error) throw new AdminError(500, existing.error.message);

	const found = existing.data as { id: string } | null;
	if (found) {
		const { error } = await db
			.from('repository_elections')
			.update(stamp())
			.eq('id', found.id);
		if (error) throw new AdminError(500, error.message);
		return;
	}

	const { error } = await db.from('repository_elections').insert({
		// NOT NULL, and not derivable from `repository_id` without a second read —
		// so it is carried from the view row rather than re-looked-up. Denormalised
		// in the DDL on purpose: it is what tenant-scoped policy reads join on.
		tenant_id: mine.tenant_id,
		repository_id: repositoryId,
		authority,
		principal_id: principalSlot,
		elected
	});
	if (!error) return;

	// 23505 — someone else inserted between our read and our insert. Their row is
	// the one that exists now, so apply our value to it.
	if (error.code !== '23505') throw new AdminError(500, error.message);
	const retry = await (principalSlot === null
		? db
				.from('repository_elections')
				.update(stamp())
				.eq('repository_id', repositoryId)
				.eq('authority', authority)
				.is('principal_id', null)
		: db
				.from('repository_elections')
				.update(stamp())
				.eq('repository_id', repositoryId)
				.eq('authority', authority)
				.eq('principal_id', principalSlot));
	if (retry.error) throw new AdminError(500, retry.error.message);
}
