// The ELECTION write path — the half of `sync_enabled` that had no writer.
//
// `dojo.all_my_repositories` computes `may_share AND elected`, and until now
// nothing anywhere wrote `dojo.repository_elections`. Every user-authority
// repository was therefore permanently `elected = false`: the view was correct
// and the answer was always no.
//
// The load-bearing design decision under every test here: **this module does not
// re-derive authorization.** Who may elect, and which authority applies, are
// read from the view — the same row the daemon, the API and the UI read. A
// second derivation is a second thing to keep in step, and the whole point of
// the view was to stop having four of them.
import { describe, it, expect, beforeEach } from 'vitest';
import { fakeDojoDb, resetFakeIds, type FakeTable } from './fake-dojo-db';
import { setElection } from './elections';
import { AdminError } from './admin-data';

const ALICE = 'p-alice';

/** One row per (principal, repo) — which is what the view is. Defaults describe
 *  a PERSONAL PRIVATE repo Alice may elect and has not. */
function viewRow(over: Record<string, unknown> = {}) {
	return {
		repository_id: 'r-1',
		tenant_id: 't-1',
		repo_key: 'github.com/alice/api',
		principal_id: ALICE,
		role: 'admin',
		authority: 'user',
		configurable_by_me: true,
		elected: false,
		may_share: true,
		sync_enabled: false,
		reason_code: 'not_elected_user',
		reason: 'You have not chosen to share this repository',
		reason_actor: 'user',
		...over
	};
}

function tables(rows: Record<string, unknown>[] = [viewRow()]): Record<string, FakeTable> {
	return {
		all_my_repositories: { rows },
		repository_elections: {
			rows: [],
			// Mirrors the DDL's `unique nulls not distinct (repository_id, authority,
			// principal_id)`. The fake compares with `===`, so NULL === NULL and it
			// behaves as NULLS NOT DISTINCT — which is what the DDL declares, and NOT
			// the Postgres default. That default is why the DDL says it explicitly:
			// an org election has a NULL principal, so under NULLS DISTINCT the
			// constraint would fire for nothing and every org write would insert a
			// SECOND row instead of updating. The real constraint is proved against
			// real Postgres in database/tests/dojo/, not here.
			uniques: [{ columns: ['repository_id', 'authority', 'principal_id'] }]
		}
	};
}

beforeEach(() => resetFakeIds());

describe('setElection — where the row lands', () => {
	it('records a USER election against the electing principal', async () => {
		const db = fakeDojoDb(tables());
		await setElection(db as never, ALICE, 'github.com/alice/api', true);
		expect(db.tables.repository_elections.rows).toHaveLength(1);
		expect(db.tables.repository_elections.rows[0]).toMatchObject({
			repository_id: 'r-1',
			// NOT NULL in the DDL. The fake accepts any column, so nothing here would
			// have failed without this assertion — the real insert would have.
			tenant_id: 't-1',
			authority: 'user',
			principal_id: ALICE,
			elected: true
		});

		// The columns written must be columns that EXIST. The fake is not a schema:
		// it accepted `elected_by`, a column I invented, and every test stayed green
		// while the real insert would have failed with "column does not exist".
		// Pinned against the live DDL for dojo.repository_elections.
		const REAL = new Set([
			'id','tenant_id','repository_id','authority','principal_id',
			'elected','elected_at','created_at','modified_at'
		]);
		const written = Object.keys(db.tables.repository_elections.rows[0]);
		expect(written.filter((c) => !REAL.has(c))).toEqual([]);
	});

	it('records an ORGANIZATION election with a NULL principal', async () => {
		// Not a detail: `repository_elections_principal_matches_authority` REQUIRES
		// principal_id IS NULL for an org election, and the view joins the org slot
		// on that. Writing the admin's own id here would insert a row the view never
		// reads — an election that appears to have been made and does nothing.
		const db = fakeDojoDb(tables([viewRow({ authority: 'organization', role: 'admin' })]));
		await setElection(db as never, ALICE, 'github.com/alice/api', true);
		expect(db.tables.repository_elections.rows[0]).toMatchObject({
			authority: 'organization',
			principal_id: null,
			elected: true
		});
	});

	it('takes the authority from the VIEW, never from the caller', async () => {
		// The security case. If the authority could be supplied, any member of an
		// org could write the ORGANIZATION's election for a repository and share it
		// on everyone's behalf. `setElection` has no parameter for it by
		// construction — this test pins that the signature stays that way.
		const db = fakeDojoDb(tables());
		await setElection(db as never, ALICE, 'github.com/alice/api', true);
		expect(db.tables.repository_elections.rows[0].authority).toBe('user');
		expect(setElection).toHaveLength(4); // db, principalId, repoKey, elected
	});
});

describe('setElection — who is refused', () => {
	it('refuses a member who may not configure it, and names who can', async () => {
		const db = fakeDojoDb(
			tables([
				viewRow({
					authority: 'organization',
					role: 'member',
					configurable_by_me: false,
					reason_actor: 'admin'
				})
			])
		);
		await expect(setElection(db as never, ALICE, 'github.com/alice/api', true)).rejects.toThrow(
			AdminError
		);
		await expect(
			setElection(db as never, ALICE, 'github.com/alice/api', true)
		).rejects.toMatchObject({ status: 403 });
		expect(db.tables.repository_elections.rows).toHaveLength(0);
	});

	it('refuses a repository whose forge visibility is not captured yet', async () => {
		// Authority is DERIVED from visibility, so with no answer there is no
		// authority — nobody holds the choice. Accepting the election would write a
		// row under a guessed authority that a later capture could contradict.
		const db = fakeDojoDb(
			tables([
				viewRow({ authority: null, configurable_by_me: false, reason_code: 'forge_visibility_unknown' })
			])
		);
		await expect(
			setElection(db as never, ALICE, 'github.com/alice/api', true)
		).rejects.toMatchObject({ status: 409 });
		expect(db.tables.repository_elections.rows).toHaveLength(0);
	});

	it('404s a repository that is not in the caller view — scoping IS the authorization', async () => {
		// The view is already per-principal, so "not in my view" and "not mine" are
		// the same fact. There is no second membership check to forget.
		const db = fakeDojoDb(tables());
		await expect(
			setElection(db as never, ALICE, 'github.com/someone-else/private', true)
		).rejects.toMatchObject({ status: 404 });
		expect(db.tables.repository_elections.rows).toHaveLength(0);
	});
});

describe('setElection — repeating it', () => {
	it('updates in place rather than accumulating rows', async () => {
		const db = fakeDojoDb(tables());
		await setElection(db as never, ALICE, 'github.com/alice/api', true);
		await setElection(db as never, ALICE, 'github.com/alice/api', false);
		expect(db.tables.repository_elections.rows).toHaveLength(1);
		expect(db.tables.repository_elections.rows[0].elected).toBe(false);
	});

	it('turning it OFF writes elected=false rather than deleting the row', async () => {
		// `configured_by`/`configured_at` distinguish "your org turned this off"
		// from "nobody has looked". Deleting the row destroys that distinction and
		// the view reports the repository as never-decided.
		const db = fakeDojoDb(tables());
		await setElection(db as never, ALICE, 'github.com/alice/api', false);
		expect(db.tables.repository_elections.rows).toHaveLength(1);
		expect(db.tables.repository_elections.rows[0]).toMatchObject({ elected: false });
	});
});

describe('setElection — what it returns', () => {
	it('returns the verdict re-read from the view, not a locally computed one', async () => {
		// Electing is only HALF the decision. A repository whose entitlement still
		// refuses must come back `sync_enabled: false`, or the UI reports success
		// and the daemon then declines to push — the exact "it says shared but
		// nothing arrives" gap the two-axis model exists to close.
		const db = fakeDojoDb(
			tables([
				viewRow({
					// what the view says AFTER the write: elected, but unsubscribed
					elected: true,
					may_share: false,
					sync_enabled: false,
					reason_code: 'not_subscribed'
				})
			])
		);
		const out = await setElection(db as never, ALICE, 'github.com/alice/api', true);
		expect(out).toMatchObject({
			repo_key: 'github.com/alice/api',
			elected: true,
			sync_enabled: false,
			reason_code: 'not_subscribed'
		});
	});
});
