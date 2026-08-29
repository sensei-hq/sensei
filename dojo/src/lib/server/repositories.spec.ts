// Repository registration and the sync plan (§V.4, in the user-scoped shape
// §VIII.1 corrected it to). Runs against `fakeDojoDb` so the (tenant_id,
// repo_key) unique is real and re-registering is observably a no-op.
import { describe, it, expect, beforeEach } from 'vitest';
import { fakeDojoDb, resetFakeIds, type FakeTable } from './fake-dojo-db';
import { registerRepositories, syncPlan } from './repositories';

const ALICE = 'p-alice';

/** Alice is in `organization/acme` (connected to github org `acme`) and in her
 *  personal dōjō. `organization/other` exists but she is not a member. */
function tables(): Record<string, FakeTable> {
	return {
		memberships: {
			rows: [
				{ id: 'm1', tenant_id: 't-acme', user_id: ALICE },
				{ id: 'm2', tenant_id: 't-personal', user_id: ALICE },
				{ id: 'm3', tenant_id: 't-other', user_id: 'p-bob' }
			]
		},
		tenants: {
			rows: [
				{ id: 't-acme', key: 'organization/acme', origin: 'organization', slug: 'acme' },
				{ id: 't-personal', key: 'personal/alice', origin: 'personal', slug: 'alice' },
				{ id: 't-other', key: 'organization/other', origin: 'organization', slug: 'other' }
			]
		},
		tenant_connections: {
			rows: [
				{ id: 'c1', tenant_id: 't-acme', provider: 'github', external_id: '11', external_slug: 'acme' },
				{ id: 'c2', tenant_id: 't-other', provider: 'github', external_id: '22', external_slug: 'secret-org' }
			]
		},
		repositories: {
			rows: [],
			uniques: [{ columns: ['tenant_id', 'repo_key'] }]
		}
	};
}

beforeEach(() => resetFakeIds());

describe('registerRepositories', () => {
	it('maps a repo to the tenant its forge org is connected to', async () => {
		const db = fakeDojoDb(tables());
		const out = await registerRepositories(db as never, ALICE, [
			{ repo_key: 'github.com/acme/api', remote_url: 'git@github.com:acme/api.git', name: 'api' }
		]);
		expect(out.unmapped).toEqual([]);
		expect(out.mapped[0]).toMatchObject({
			repo_key: 'github.com/acme/api',
			tenant: 'organization/acme',
			// The uuid the daemon stores in sensei.repositories.tenant_id (D2).
			// `tenant` above is a display key and a rename changes it.
			tenant_id: 't-acme'
		});
		expect(db.tables.repositories.rows[0]).toMatchObject({
			tenant_id: 't-acme',
			repo_key: 'github.com/acme/api',
			name: 'api',
			// Stored, not re-derived later: dojo.repositories.provider is NOT NULL,
			// and keeping the host→provider mapping in one place is why.
			provider: 'github'
		});
	});

	it('is idempotent — re-registering returns the same row, not a second one', async () => {
		const db = fakeDojoDb(tables());
		const first = await registerRepositories(db as never, ALICE, [
			{ repo_key: 'github.com/acme/api' }
		]);
		const second = await registerRepositories(db as never, ALICE, [
			{ repo_key: 'github.com/acme/api' }
		]);
		expect(db.tables.repositories.rows).toHaveLength(1);
		expect(second.mapped[0].repo_id).toBe(first.mapped[0].repo_id);
	});

	it('derives a name from the key when the daemon sent none', async () => {
		// `name` is NOT NULL. The last segment IS the repository's name, so this
		// is derived from the identity we were given, not invented.
		const db = fakeDojoDb(tables());
		await registerRepositories(db as never, ALICE, [{ repo_key: 'github.com/acme/deep-repo' }]);
		expect(db.tables.repositories.rows[0].name).toBe('deep-repo');
	});

	it('leaves an unrecognised host UNMAPPED, and writes no row', async () => {
		// §II.6: unmapped is NOT personal. Defaulting a self-hosted remote to the
		// caller's personal dōjō would move an employer's private repository into
		// a free personal tenant.
		const db = fakeDojoDb(tables());
		const out = await registerRepositories(db as never, ALICE, [
			{ repo_key: 'git.internal.acme.com/acme/api' }
		]);
		expect(out.mapped).toEqual([]);
		expect(out.unmapped).toEqual([{ repo_key: 'git.internal.acme.com/acme/api', reason: 'unknown_host' }]);
		expect(db.tables.repositories.rows).toHaveLength(0);
	});

	it('reports no_connection when the org has never been connected', async () => {
		const db = fakeDojoDb(tables());
		const out = await registerRepositories(db as never, ALICE, [
			{ repo_key: 'github.com/never-seen/api' }
		]);
		expect(out.unmapped).toEqual([{ repo_key: 'github.com/never-seen/api', reason: 'no_connection' }]);
	});

	it('refuses to register into a tenant the caller does not belong to', async () => {
		// The authorization boundary. `secret-org` IS connected — to a tenant
		// Alice is not in. Without this check anyone could push repositories into
		// any tenant that happened to have connected their org.
		const db = fakeDojoDb(tables());
		const out = await registerRepositories(db as never, ALICE, [
			{ repo_key: 'github.com/secret-org/private-api' }
		]);
		expect(out.unmapped).toEqual([
			{ repo_key: 'github.com/secret-org/private-api', reason: 'not_a_member' }
		]);
		expect(db.tables.repositories.rows).toHaveLength(0);
	});

	it('reports ambiguous rather than guessing when a slug resolves to two of my tenants', async () => {
		// Only possible when an org was renamed and the name re-registered
		// upstream. Picking one would attach real code to the wrong governance
		// boundary — a silent, invisible mis-routing.
		const t = tables();
		t.tenant_connections.rows.push({
			id: 'c3',
			tenant_id: 't-personal',
			provider: 'github',
			external_id: '33',
			external_slug: 'ACME' // case-insensitively the same slug
		});
		const db = fakeDojoDb(t);
		const out = await registerRepositories(db as never, ALICE, [{ repo_key: 'github.com/acme/api' }]);
		expect(out.unmapped).toEqual([{ repo_key: 'github.com/acme/api', reason: 'ambiguous' }]);
		expect(db.tables.repositories.rows).toHaveLength(0);
	});

	it('handles a mixed batch without letting one bad repo stop the rest', async () => {
		const db = fakeDojoDb(tables());
		const out = await registerRepositories(db as never, ALICE, [
			{ repo_key: 'github.com/acme/api' },
			{ repo_key: 'git.internal/x/y' },
			{ repo_key: 'github.com/acme/web' }
		]);
		expect(out.mapped.map((m) => m.repo_key)).toEqual([
			'github.com/acme/api',
			'github.com/acme/web'
		]);
		expect(out.unmapped).toHaveLength(1);
	});
});

describe('syncPlan', () => {
	// syncPlan reads `dojo.all_my_repositories`, which already joins repository →
	// tenant → membership and carries the owning tenant on each row. The view's
	// own semantics (membership scoping, disabled memberships) are covered by
	// database/tests/dojo/all_my_repositories.sql, against a real Postgres —
	// a fake cannot evaluate a view. What is under test here is the split.
	function withView(rows: Record<string, unknown>[]): Record<string, FakeTable> {
		return { ...tables(), all_my_repositories: { rows } };
	}

	const MINE = [
		{
			repository_id: 'r1',
			repo_key: 'github.com/acme/api',
			tenant: 'organization/acme',
			tenant_id: 't-acme',
			principal_id: ALICE,
			sync_enabled: true
		},
		{
			repository_id: 'r2',
			repo_key: 'github.com/acme/web',
			tenant: 'organization/acme',
			tenant_id: 't-acme',
			principal_id: ALICE,
			sync_enabled: true
		}
	];

	it('allows every registered repo in phase 1, with denied present but empty', async () => {
		// `denied: []` rather than absent, so the daemon's handling of it is
		// exercised from day one and phase 2 changes no shape (§V.5).
		const db = fakeDojoDb(withView(MINE));
		const plan = await syncPlan(db as never, ALICE);
		expect(plan.allowed.map((a) => a.repo_key).sort()).toEqual([
			'github.com/acme/api',
			'github.com/acme/web'
		]);
		expect(plan.allowed[0].tenant).toBe('organization/acme');
		expect(plan.denied).toEqual([]);
	});

	it('carries tenant_id, not just the display key, so the daemon can store it', async () => {
		// `sensei.repositories.tenant_id` holds a uuid (daemon-sync.md D2). Without
		// this the daemon has only `organization/acme` — a DISPLAY key that a
		// rename changes — and would have to either store the wrong thing or
		// re-resolve the tenant on every cycle.
		const db = fakeDojoDb(withView(MINE));
		const plan = await syncPlan(db as never, ALICE);
		expect(plan.allowed[0].tenant_id).toBe('t-acme');
	});

	it('scopes to the caller, so one user never sees another user rows', async () => {
		// The plan is an ALLOW-LIST the daemon acts on directly, so a leak here is
		// not a display bug — it is the daemon syncing someone else code.
		const db = fakeDojoDb(
			withView([
				...MINE,
				{
					repository_id: 'r-other',
					repo_key: 'github.com/secret-org/private-api',
					tenant: 'organization/other',
					tenant_id: 't-other',
					principal_id: 'p-bob',
					sync_enabled: true
				}
			])
		);
		const plan = await syncPlan(db as never, ALICE);
		expect(plan.allowed.map((a) => a.repo_key)).not.toContain('github.com/secret-org/private-api');
	});

	it('splits on sync_enabled, so the phase-2 gate needs no code change here', async () => {
		// The view computes sync_enabled: TRUE throughout phase 1, the can_sync
		// predicate in phase 2. Reading it rather than assuming it is what lets the
		// gate arrive as a view change.
		const db = fakeDojoDb(
			withView([
				MINE[0],
				{ ...MINE[1], sync_enabled: false, denied_reason: 'no_seat' }
			])
		);
		const plan = await syncPlan(db as never, ALICE);
		expect(plan.allowed.map((a) => a.repo_key)).toEqual(['github.com/acme/api']);
		expect(plan.denied).toEqual([
			{ repo_key: 'github.com/acme/web', tenant: 'organization/acme', reason: 'no_seat' }
		]);
	});

	it('returns an empty plan for a user with no repositories', async () => {
		// Genuinely empty: nothing of theirs to sync. Offline degrades the same way
		// by construction — no plan, no sync.
		const db = fakeDojoDb(withView([]));
		expect(await syncPlan(db as never, 'p-nobody')).toEqual({ allowed: [], denied: [] });
	});
});

// ── The READ side of sharing ────────────────────────────────────────────────
//
// `all_my_repositories` had a daemon reader (`syncPlan`) and a writer
// (`setElection`) and no way for a HUMAN to see it. Live, three repositories sat
// at `not_elected_user` — refusing for want of a decision nobody could make,
// because nothing rendered a toggle.
//
// This returns the row the screen needs, from the same view the daemon reads, so
// what a user is shown and what the daemon does cannot disagree.
import { listMyRepositories } from './repositories';

describe('listMyRepositories — what a human is shown', () => {
	function view(rows: Record<string, unknown>[]): Record<string, FakeTable> {
		return { all_my_repositories: { rows } };
	}
	const ROW = {
		repository_id: 'r1',
		repo_key: 'github.com/acme/api',
		name: 'api',
		tenant: 'organization/acme',
		owning_org: 'acme',
		principal_id: ALICE,
		forge_visibility: 'public',
		authority: 'user',
		may_share: true,
		elected: false,
		sync_enabled: false,
		configurable_by_me: true,
		reason_code: 'not_elected_user',
		reason: 'You have not turned sharing on for this repository',
		remedy: 'Turn sharing on for this repository',
		reason_actor: 'user',
		last_synced_at: null,
		metric_rows: 0
	};

	it('returns only the caller rows — the view is per-principal', async () => {
		const db = fakeDojoDb(view([ROW, { ...ROW, repo_key: 'github.com/acme/other', principal_id: 'p-bob' }]));
		const out = await listMyRepositories(db as never, ALICE);
		expect(out.map((r) => r.repo_key)).toEqual(['github.com/acme/api']);
	});

	it('carries the verdict AND the remedy, so a refusal names what to do', async () => {
		// The whole point of the reason registry. A screen that shows only
		// `sync_enabled: false` reproduces the "nothing to sync" ambiguity the
		// two-axis model exists to remove.
		const db = fakeDojoDb(view([ROW]));
		const [r] = await listMyRepositories(db as never, ALICE);
		expect(r).toMatchObject({
			sync_enabled: false,
			reason_code: 'not_elected_user',
			remedy: 'Turn sharing on for this repository',
			reason_actor: 'user',
			configurable_by_me: true
		});
	});

	it('THROWS on a read error rather than reporting an empty list', async () => {
		// An empty list reads as "you have no repositories", which is a different
		// and load-bearing claim. Same fail-closed rule as `listUserOrgs`.
		//
		// `fakeDojoDb` cannot fail a read, so this uses a minimal chainable stub —
		// the failure path is worth a bespoke double rather than going untested.
		const failing = {
			from: () => failing,
			select: () => failing,
			eq: () => Promise.resolve({ data: null, error: { message: 'boom' } })
		} as unknown as Parameters<typeof listMyRepositories>[0];
		await expect(listMyRepositories(failing, ALICE)).rejects.toThrow();
	});
});

// ── PostgREST silently caps every read at 1000 rows ─────────────────────────
//
// `PGRST_DB_MAX_ROWS=1000` — verified on the running instance, not assumed from
// a default. A read with no `.range()` therefore returns AT MOST 1000 rows and
// says nothing about it: there is no error, no flag, and the 1001st row simply
// does not exist as far as the caller is concerned.
//
// That is silent truncation on the three reads of `all_my_repositories`, and the
// ingest one is the worst of them: `maySync` would be built from a truncated
// allow-list, so a repository the user IS permitted to sync gets refused as
// `not_permitted` — a denial that names the wrong reason.
//
// The daemon already offers up to 500 repositories per pass and a developer
// machine here carries 67, so this is a ceiling being approached, not a
// hypothetical.
describe('paged reads — the 1000-row cap must not truncate silently', () => {
	function manyRows(n: number) {
		return Array.from({ length: n }, (_, i) => ({
			repository_id: `r${i}`,
			repo_key: `github.com/acme/repo-${i}`,
			name: `repo-${i}`,
			tenant: 'organization/acme',
			owning_org: 'acme',
			principal_id: ALICE,
			forge_visibility: 'public',
			authority: 'user',
			may_share: true,
			elected: true,
			sync_enabled: true,
			configurable_by_me: true,
			reason_code: null,
			reason: null,
			remedy: null,
			reason_actor: null,
			last_synced_at: null,
			metric_rows: 0
		}));
	}

	it('returns EVERY repository past the first page, not the first 1000', async () => {
		const db = fakeDojoDb({ all_my_repositories: { rows: manyRows(2350) } });
		const out = await listMyRepositories(db as never, ALICE);
		expect(out).toHaveLength(2350);
		// The last row is the one a single unpaged read loses.
		expect(out.at(-1)?.repo_key).toBe('github.com/acme/repo-2349');
	});

	it('stops at exactly one page when the total IS the page size', async () => {
		// The off-by-one that turns a page loop into an infinite one: a full final
		// page looks identical to "there is more".
		const db = fakeDojoDb({ all_my_repositories: { rows: manyRows(1000) } });
		const out = await listMyRepositories(db as never, ALICE);
		expect(out).toHaveLength(1000);
	});

	it('syncPlan pages too — a truncated plan silently under-syncs', async () => {
		const db = fakeDojoDb({ all_my_repositories: { rows: manyRows(1200) } });
		const plan = await syncPlan(db as never, ALICE);
		expect(plan.allowed).toHaveLength(1200);
	});
});
