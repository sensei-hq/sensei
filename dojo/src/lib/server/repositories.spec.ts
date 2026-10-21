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
		'rpc:my_graph_branches': { rows: [] },
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
		},
		// Declared empty in the shared set because that is the DEFAULT state and
		// the one every dōjō starts in: no rows means the whole catalogue is on.
		// The fake refuses undeclared tables, which is what caught syncPlan's new
		// read — so the fake has to mirror the schema, not just the rows a test
		// happens to care about.
		metric_activations: {
			rows: [],
			uniques: [{ columns: ['tenant_id', 'repository_id', 'metric_id'] }]
		},
		metric_catalogue: { rows: [] }
	};
}

beforeEach(() => resetFakeIds());

// The MAPPING — which tenant a repository belongs to, membership, ambiguity, a
// revoked or other disabled membership, idempotency — is the database's:
// dojo.register_repository decides it, proved against a real Postgres in
// database/tests/dojo/register_repository_access.sql (#236). What is left here
// is this side's half: parsing the key, what it asks, and what it reports.
describe('registerRepositories', () => {
	type Call = { fn: string; args: Record<string, unknown> };
	function rpcDb(respond: (args: Record<string, unknown>) => { data: unknown; error: unknown }) {
		const calls: Call[] = [];
		const db = {
			rpc: (fn: string, args: Record<string, unknown>) => {
				calls.push({ fn, args });
				const answer = () => (fn === 'file_repository_project' ? fileAnswer(args) : respond(args));
				// `.single()` for a row-returning function, awaited directly for a
				// scalar one — as supabase-js offers both.
				return {
					single: async () => answer(),
					then: (ok: (v: unknown) => unknown, bad?: (e: unknown) => unknown) =>
						Promise.resolve(answer()).then(ok, bad)
				};
			}
		};
		return { db: db as never, calls };
	}
	let fileAnswer: (args: Record<string, unknown>) => { data: unknown; error: unknown } = (args) => ({
		data: `p-${args.p_slug}`,
		error: null
	});
	const filed = (args: Record<string, unknown>) => ({
		data: { tenant_id: 't-acme', tenant_key: 'organization/acme', repository_id: `r-${args.p_repo_key}`, reason: null },
		error: null
	});

	it('asks the database to file the repository under the forge org it parses', async () => {
		const { db, calls } = rpcDb(filed);
		const out = await registerRepositories(db, [
			{ repo_key: '  GitHub.com/Acme/API ', remote_url: 'git@github.com:acme/api.git', name: 'api' }
		]);
		expect(calls).toEqual([
			{
				fn: 'register_repository',
				args: {
					p_repo_key: 'github.com/acme/api',
					p_provider: 'github',
					p_org: 'acme',
					p_name: 'api',
					p_remote_url: 'git@github.com:acme/api.git'
				}
			}
		]);
		expect(out.unmapped).toEqual([]);
		expect(out.mapped).toEqual([
			{
				repo_key: 'github.com/acme/api',
				tenant: 'organization/acme',
				// The uuid the daemon stores in sensei.repositories.tenant_id (D2).
				// `tenant` above is a display key and a rename changes it.
				tenant_id: 't-acme',
				repo_id: 'r-github.com/acme/api'
			}
		]);
	});

	it('derives a name from the key when the daemon sent none', async () => {
		// `name` is NOT NULL. The last segment IS the repository's name, so this
		// is derived from the identity we were given, not invented.
		const { db, calls } = rpcDb(filed);
		await registerRepositories(db, [{ repo_key: 'github.com/acme/deep-repo' }]);
		expect(calls[0].args.p_name).toBe('deep-repo');
	});

	it('leaves an unrecognised host UNMAPPED, and never asks the database', async () => {
		// §II.6: unmapped is NOT personal. Defaulting a self-hosted remote to the
		// caller's personal dōjō would move an employer's private repository into
		// a free personal tenant.
		const { db, calls } = rpcDb(filed);
		const out = await registerRepositories(db, [{ repo_key: 'git.internal.acme.com/acme/api' }]);
		expect(out.mapped).toEqual([]);
		expect(out.unmapped).toEqual([{ repo_key: 'git.internal.acme.com/acme/api', reason: 'unknown_host' }]);
		expect(calls).toHaveLength(0);
	});

	for (const reason of ['no_connection', 'not_a_member', 'ambiguous'] as const) {
		it(`reports the database's ${reason} as unmapped, with no repository`, async () => {
			const { db } = rpcDb(() => ({
				data: { tenant_id: null, tenant_key: null, repository_id: null, reason },
				error: null
			}));
			const out = await registerRepositories(db, [{ repo_key: 'github.com/acme/api' }]);
			expect(out.mapped).toEqual([]);
			expect(out.unmapped).toEqual([{ repo_key: 'github.com/acme/api', reason }]);
		});
	}

	it('handles a mixed batch without letting one bad repo stop the rest', async () => {
		const { db } = rpcDb(filed);
		const out = await registerRepositories(db, [
			{ repo_key: 'github.com/acme/api' },
			{ repo_key: 'git.internal/x/y' },
			{ repo_key: 'github.com/acme/web' }
		]);
		expect(out.mapped.map((m) => m.repo_key)).toEqual(['github.com/acme/api', 'github.com/acme/web']);
		expect(out.unmapped).toHaveLength(1);
	});

	// #299 (decided: one dōjō project per sensei project): the daemon sends each
	// repository's sensei project, and a mapped repository is filed under it.
	const PROJECT = { slug: 'sensei', name: 'sensei' };

	it('files a mapped repository under the project the daemon sent', async () => {
		const { db, calls } = rpcDb(filed);
		const out = await registerRepositories(db, [{ repo_key: 'github.com/acme/api', project: PROJECT }]);
		expect(calls[1]).toEqual({
			fn: 'file_repository_project',
			// No classification: the database derives it from where the repository lives.
			args: { p_repository_id: 'r-github.com/acme/api', p_slug: 'sensei', p_name: 'sensei' }
		});
		expect(out.mapped[0]).toMatchObject({ repo_key: 'github.com/acme/api', project_id: 'p-sensei' });
	});

	it('files nothing for a repository sent without a project', async () => {
		const { db, calls } = rpcDb(filed);
		const out = await registerRepositories(db, [{ repo_key: 'github.com/acme/api' }]);
		expect(calls.map((c) => c.fn)).toEqual(['register_repository']);
		expect(out.mapped[0].project_id).toBeUndefined();
	});

	it('files nothing for a repository that did not map', async () => {
		const { db, calls } = rpcDb(() => ({
			data: { tenant_id: null, tenant_key: null, repository_id: null, reason: 'no_connection' },
			error: null
		}));
		await registerRepositories(db, [{ repo_key: 'github.com/acme/api', project: PROJECT }]);
		expect(calls.map((c) => c.fn)).toEqual(['register_repository']);
	});

	it('reports a filing that failed on that repository, without hiding the registration', async () => {
		const saved = fileAnswer;
		fileAnswer = () => ({ data: null, error: { message: 'not a member of this repository\'s tenant' } });
		try {
			const { db } = rpcDb(filed);
			const out = await registerRepositories(db, [{ repo_key: 'github.com/acme/api', project: PROJECT }]);
			expect(out.mapped[0]).toMatchObject({ repo_key: 'github.com/acme/api', project_error: "not a member of this repository's tenant" });
			expect(out.mapped[0].project_id).toBeUndefined();
		} finally {
			fileAnswer = saved;
		}
	});

	it('THROWS on a database error rather than reporting the repository unmapped', async () => {
		// An error is not a reason: reporting it as `no_connection` would tell the
		// person to connect an org that IS connected.
		const { db } = rpcDb(() => ({ data: null, error: { message: 'connection reset' } }));
		await expect(registerRepositories(db, [{ repo_key: 'github.com/acme/api' }])).rejects.toThrow(
			/connection reset/
		);
	});
});

describe('syncPlan', () => {
	// syncPlan reads `dojo.all_my_repositories` through dojo.my_repositories(),
	// which pins the caller; the view already joins repository →
	// tenant → membership and carries the owning tenant on each row. The view's
	// own semantics (membership scoping, disabled memberships) are covered by
	// database/tests/dojo/all_my_repositories.sql, against a real Postgres —
	// a fake cannot evaluate a view. What is under test here is the split.
	function withView(rows: Record<string, unknown>[]): Record<string, FakeTable> {
		return { ...tables(), 'rpc:my_repositories': { rows } };
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
		const plan = await syncPlan(db as never);
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
		const plan = await syncPlan(db as never);
		expect(plan.allowed[0].tenant_id).toBe('t-acme');
	});

	it('carries each repository\'s tracked graph branches, so the daemon pushes only checkouts on them', async () => {
		// graph-sync.md §9: the dōjō says which branches; the daemon contributes a
		// checkout only when its branch is one of them. A repository tracking none
		// carries [] — its graph does not sync — never a guessed main.
		const db = fakeDojoDb({
			...withView(MINE),
			'rpc:my_graph_branches': {
				rows: [
					{ repository_id: 'r1', branch: 'production', is_default: false },
					{ repository_id: 'r1', branch: 'main', is_default: true }
				]
			}
		});
		const plan = await syncPlan(db as never);
		const api = plan.allowed.find((a) => a.repo_key === 'github.com/acme/api');
		const web = plan.allowed.find((a) => a.repo_key === 'github.com/acme/web');
		expect(api?.graph_branches).toEqual([
			{ branch: 'main', is_default: true },
			{ branch: 'production', is_default: false }
		]);
		expect(web?.graph_branches).toEqual([]);
	});

	it('carries the metrics a tenant has switched OFF for that repository', async () => {
		// Absence = enabled, so the plan sends the DISABLED set. Sending "enabled"
		// instead would make a metric added to the catalogue later arrive OFF for
		// every existing repository, because no row would mention it.
		const db = fakeDojoDb({
			...withView(MINE),
			metric_activations: {
				rows: [
					{ tenant_id: 't-acme', repository_id: 'r1', metric_id: 'm-ftr', enabled: false },
					// enabled:true rows are the same as absence and must not be sent.
					{ tenant_id: 't-acme', repository_id: 'r1', metric_id: 'm-churn', enabled: true }
				]
			},
			metric_catalogue: { rows: [{ id: 'm-ftr', key: 'ftr' }, { id: 'm-churn', key: 'churn_rate' }] }
		});
		const plan = await syncPlan(db as never);
		const api = plan.allowed.find((a) => a.repo_key === 'github.com/acme/api');
		const web = plan.allowed.find((a) => a.repo_key === 'github.com/acme/web');
		expect(api?.disabled_metrics).toEqual(['ftr']);
		// Per REPOSITORY: r2 was never mentioned, so nothing is off for it.
		expect(web?.disabled_metrics).toEqual([]);
	});

	it('reports metric KEYS, not ids, because the daemon knows keys', async () => {
		// `sensei.metrics.id` differs between the two planes — they are separate
		// databases loaded from the same staging file — so sending the uuid would
		// name a row the daemon cannot resolve. `key` is the stable slug.
		const db = fakeDojoDb({
			...withView(MINE),
			metric_activations: {
				rows: [{ tenant_id: 't-acme', repository_id: 'r1', metric_id: 'm-ftr', enabled: false }]
			},
			metric_catalogue: { rows: [{ id: 'm-ftr', key: 'ftr' }] }
		});
		const plan = await syncPlan(db as never);
		const api = plan.allowed.find((a) => a.repo_key === 'github.com/acme/api');
		expect(api?.disabled_metrics).toEqual(['ftr']);
	});

	it('propagates an activations read failure instead of re-enabling everything', async () => {
		// The wrong direction to fail for a cost lever. An empty map on error looks
		// exactly like "nothing is disabled", so a broken read would silently start
		// computing every metric a tenant is paying NOT to compute — and the only
		// symptom would be the bill.
		const db = fakeDojoDb({
			...withView(MINE),
			metric_activations: { rows: [], error: { message: 'connection reset' } }
		});
		await expect(syncPlan(db as never)).rejects.toThrow(/connection reset/);
	});

	it('leaves disabled_metrics empty when nothing was ever switched off', async () => {
		// The overwhelmingly common case, and the one a new dōjō is in: no rows,
		// whole catalogue on.
		const db = fakeDojoDb(withView(MINE));
		const plan = await syncPlan(db as never);
		expect(plan.allowed.every((a) => a.disabled_metrics.length === 0)).toBe(true);
	});

	// The plan is an ALLOW-LIST the daemon acts on directly, so a leak here is not
	// a display bug — it is the daemon syncing someone else's code. The scoping is
	// the DATABASE's: dojo.my_repositories() pins the caller's principal (proved in
	// database/tests/dojo/my_repositories_access.sql). What this side must not do
	// is read the view directly, where the boundary is a filter one caller forgets.
	it('reads through dojo.my_repositories(), never the view', async () => {
		const db = fakeDojoDb(withView(MINE)); // no `all_my_repositories` table: a direct read throws
		const plan = await syncPlan(db as never);
		expect(plan.allowed.map((a) => a.repo_key).sort()).toEqual(['github.com/acme/api', 'github.com/acme/web']);
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
		const plan = await syncPlan(db as never);
		expect(plan.allowed.map((a) => a.repo_key)).toEqual(['github.com/acme/api']);
		expect(plan.denied).toEqual([
			{ repo_key: 'github.com/acme/web', tenant: 'organization/acme', reason: 'no_seat' }
		]);
	});

	it('returns an empty plan for a user with no repositories', async () => {
		// Genuinely empty: nothing of theirs to sync. Offline degrades the same way
		// by construction — no plan, no sync.
		const db = fakeDojoDb(withView([]));
		expect(await syncPlan(db as never)).toEqual({ allowed: [], denied: [] });
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
		return { 'rpc:my_repositories': { rows } };
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

	// Scoping to the caller is dojo.my_repositories()'s (proved in
	// database/tests/dojo/my_repositories_access.sql); this side reads through it.
	it('reads through dojo.my_repositories(), never the view', async () => {
		const db = fakeDojoDb(view([ROW])); // no `all_my_repositories` table: a direct read throws
		const out = await listMyRepositories(db as never);
		expect(out.map((r) => r.repo_key)).toEqual(['github.com/acme/api']);
	});

	it('carries the verdict AND the remedy, so a refusal names what to do', async () => {
		// The whole point of the reason registry. A screen that shows only
		// `sync_enabled: false` reproduces the "nothing to sync" ambiguity the
		// two-axis model exists to remove.
		const db = fakeDojoDb(view([ROW]));
		const [r] = await listMyRepositories(db as never);
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
		// `fakeDojoDb` can fail a read now (FakeTable.error), so this uses the one
		// mechanism rather than a bespoke chainable stub. The stub had to mirror
		// the client's chain by hand, which meant it silently stopped covering
		// anything the real chain grew.
		const db = fakeDojoDb({
			...tables(),
			'rpc:my_repositories': { rows: [], error: { message: 'boom' } }
		});
		await expect(listMyRepositories(db as never)).rejects.toThrow(/boom/);
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
		const db = fakeDojoDb({ 'rpc:my_repositories': { rows: manyRows(2350) } });
		const out = await listMyRepositories(db as never);
		expect(out).toHaveLength(2350);
		// The last row is the one a single unpaged read loses.
		expect(out.at(-1)?.repo_key).toBe('github.com/acme/repo-2349');
	});

	it('stops at exactly one page when the total IS the page size', async () => {
		// The off-by-one that turns a page loop into an infinite one: a full final
		// page looks identical to "there is more".
		const db = fakeDojoDb({ 'rpc:my_repositories': { rows: manyRows(1000) } });
		const out = await listMyRepositories(db as never);
		expect(out).toHaveLength(1000);
	});

	it('syncPlan pages too — a truncated plan silently under-syncs', async () => {
		// `tables()` supplies the empty metric_activations/metrics the plan reads.
		// Without them the fake refuses the read and this fails for a reason that
		// has nothing to do with paging.
		const db = fakeDojoDb({ ...tables(), 'rpc:my_repositories': { rows: manyRows(1200) } });
		const plan = await syncPlan(db as never);
		expect(plan.allowed).toHaveLength(1200);
	});
});
