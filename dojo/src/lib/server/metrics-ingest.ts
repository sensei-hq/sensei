// The receiving half of the metric push (daemon-sync.md §7).
//
// The daemon holds a plan (`GET /v1/you/sync/plan`) listing what it may sync,
// but entitlement is RE-DECIDED here on every write. That is deliberate and is
// stated on the plan endpoint itself: the plan stops the daemon shipping data
// that would be refused, so ignoring it gains a daemon nothing. A plan is an
// optimisation; this is the boundary.
//
// WHY METRICS ARE ADDRESSED BY KEY, NOT BY UUID. The daemon sends
// `metric: 'commits_per_day'` and this resolves it against `sensei.metrics`.
// Both planes deploy the `sensei` schema, so the uuids are *probably* identical
// — but "probably identical across every install" is not an identity, and a
// mismatch would attach real numbers to the wrong metric silently. The key is
// the stable identity; an unresolvable one is rejected, never defaulted.
import { AdminError, type DojoClient } from './admin-data';

/** One metric row the daemon is offering. */
export interface MetricInput {
	repo_key: string;
	/** `sensei.metrics.key` — not a uuid. See the module header. */
	metric: string;
	scope: string;
	computed_on: string;
	grain: string;
	value: number;
	commit_sha?: string | null;
	props?: Record<string, unknown> | null;
	source?: string | null;
}

/** Why a row was not stored. Each is a different problem with different advice,
 *  so they are reported per row rather than collapsed into a count. */
export type RejectReason =
	// Covers BOTH "no such repo_key anywhere" and "registered only in a tenant you
	// are not in". Deliberately one reason, not two: distinguishing them is an
	// existence oracle over every tenant's repositories, probeable MAX_ROWS at a
	// time by any signed-up stranger. There is no `not_permitted` here for that
	// reason.
	| 'unknown_repository'
	// The repository IS yours and the dōjō refused it anyway — unelected,
	// unsubscribed, uncaptured. NOT the oracle the earlier `not_permitted` was: that
	// one distinguished "registered in a tenant you are not in" from "registered
	// nowhere", which let a stranger probe other orgs. This fires only AFTER the
	// repository has resolved inside the caller's own tenants, so it reveals nothing
	// they did not already know. `dojo.all_my_repositories.reason_code` carries the
	// specific why.
	| 'not_permitted'
	| 'unknown_metric' //     no sensei.metrics row has that key
	| 'ambiguous' //          that repo_key is registered under two of the caller's tenants
	| 'unsupported_scope'; // scope=user, which has no principal to attribute to

export interface IngestResult {
	accepted: number;
	rejected: { repo_key: string; metric: string; reason: RejectReason }[];
}

/**
 * Store pushed metric rows, re-checking entitlement per row.
 *
 * **`scope='user'` is refused, loudly.** `dojo.repository_metrics.principal_id`
 * is a PRINCIPAL, never a git email — commit trailers are unverified, so
 * accepting one would let anyone attribute work to a colleague. The bridge is
 * `sensei.personas.principal_id`, which is NULL until a persona is linked, so
 * there is currently nothing honest to attribute a user-scoped row to. Rejecting
 * names the gap; silently skipping would report a successful push that quietly
 * moved thousands of rows fewer than the daemon offered.
 */
export async function ingestMetrics(
	db: DojoClient,
	principalId: string,
	rows: MetricInput[]
): Promise<IngestResult> {
	const rejected: IngestResult['rejected'] = [];
	let accepted = 0;
	if (rows.length === 0) return { accepted, rejected };

	// The caller's tenants — the authorization boundary, read once for the batch.
	//
	// A REVOKED membership is no membership. This read used to omit `disabled_at`
	// and relied, unknowingly, on the view gate below to catch an ex-member's
	// push. The two checks answer different questions — this one asks whether the
	// caller is anyone here at all, the view asks what they are entitled to — so
	// each has to hold on its own rather than one covering for the other.
	const mem = await db
		.from('memberships')
		.select('tenant_id')
		.eq('user_id', principalId)
		.is('disabled_at', null);
	if (mem.error) throw new AdminError(500, mem.error.message);
	const mine = new Set(((mem.data ?? []) as { tenant_id: string }[]).map((m) => m.tenant_id));

	// BATCHED, not per row. This loop used to issue ~4 PostgREST subrequests per
	// row inside ONE Cloudflare Worker invocation: at the 500 rows the daemon sends
	// that is ~2001 sequential round trips, past Cloudflare's per-invocation
	// subrequest cap and past the daemon's 30s timeout. The live run that "worked"
	// moved 132 rows — a quarter of the load — and a failure here is not a clean
	// one: the Worker has already committed a prefix, the daemon marks nothing
	// shared, and the identical window retries every cadence.
	//
	// Two `in`-list reads resolve the whole batch. The per-row work that remains is
	// the existing-row check and the write, which are genuinely per row.
	// THE GATE. The daemon no longer filters the push locally (B2) on the premise
	// that the dōjō re-decides entitlement at the write — and until this existed
	// that premise was false: membership was the only check, so a member could push
	// metrics for a repository that is unelected, unsubscribed, or whose forge
	// visibility was never captured.
	//
	// `dojo.all_my_repositories` is the single source of truth for that verdict, so
	// this reads it rather than re-deriving `may_share AND elected` here. A second
	// derivation is exactly what the view exists to prevent.
	const wantedKeys = [...new Set(rows.map((r) => r.repo_key))];

	// Scoped to THIS BATCH, not to the account. PostgREST caps an unbounded read
	// at `max_rows` (1000, verified live) and says nothing about truncating — so
	// reading every repository the caller can see would, past that ceiling,
	// silently drop rows from the allow-list and refuse a repository the user IS
	// entitled to sync, with `not_permitted` naming the wrong reason.
	//
	// Filtering by the pushed keys also makes the read proportional to the push
	// rather than to the account, which is why the ceiling stops applying: a
	// batch is capped at MAX_ROWS metrics, so `wantedKeys` cannot exceed it.
	const permitted = await db
		.from('all_my_repositories')
		.select('repo_key, sync_enabled')
		.eq('principal_id', principalId)
		.in('repo_key', wantedKeys);
	if (permitted.error) throw new AdminError(500, permitted.error.message);
	const maySync = new Set(
		((permitted.data ?? []) as { repo_key: string; sync_enabled: boolean }[])
			.filter((p) => p.sync_enabled)
			.map((p) => p.repo_key)
	);
	const wantedMetrics = [...new Set(rows.map((r) => r.metric))];

	const repoRows = await db
		.from('repositories')
		.select('id, tenant_id, repo_key')
		.in('repo_key', wantedKeys);
	if (repoRows.error) throw new AdminError(500, repoRows.error.message);
	const byKey = new Map<string, { id: string; tenant_id: string }[]>();
	for (const r of (repoRows.data ?? []) as { id: string; tenant_id: string; repo_key: string }[]) {
		const list = byKey.get(r.repo_key) ?? [];
		list.push({ id: r.id, tenant_id: r.tenant_id });
		byKey.set(r.repo_key, list);
	}

	const metricRows = await db
		.from('metric_catalogue')
		.select('id, key')
		.in('key', wantedMetrics);
	if (metricRows.error) throw new AdminError(500, metricRows.error.message);
	const metricIds = new Map(
		((metricRows.data ?? []) as { id: string; key: string }[]).map((m) => [m.key, m.id])
	);

	for (const row of rows) {
		const reject = (reason: RejectReason) =>
			rejected.push({ repo_key: row.repo_key, metric: row.metric, reason });

		if (row.scope !== 'repo') {
			reject('unsupported_scope');
			continue;
		}

		// From the batch map. `dojo.repositories` is `unique (tenant_id, repo_key)`
		// and its DDL says so deliberately: "ONE ROW PER (repo_key, tenant), not one
		// globally. A consultant legitimately has the same repository under two
		// clients." So a key can legitimately return SEVERAL rows — an unscoped
		// `.maybeSingle()` here returned PGRST116 the moment it did, throwing a 500
		// that killed the entire batch, permanently.
		const all = byKey.get(row.repo_key) ?? [];
		if (all.length === 0) {
			reject('unknown_repository');
			continue;
		}
		const ours = all.filter((r) => mine.has(r.tenant_id));
		if (ours.length === 0) {
			// `unknown_repository`, NOT `not_permitted`, and deliberately identical to
			// the never-registered case above.
			//
			// Distinguishing them is an EXISTENCE ORACLE: signup is open, so any
			// stranger could post up to MAX_ROWS probe rows per request and learn
			// which private repo_keys other organisations have enrolled. An earlier
			// comment here claimed the opposite — that conflating them would leak —
			// which had it exactly backwards.
			//
			// The cost is a legitimate caller cannot tell "not yours" from "not
			// known". They can: it is in their own tenant list either way.
			reject('unknown_repository');
			continue;
		}
		// Checked AFTER the repository resolves, so a repo_key that is simply unknown
		// still answers `unknown_repository` rather than being masked as not-permitted.
		if (!maySync.has(row.repo_key)) {
			reject('not_permitted');
			continue;
		}
		if (ours.length > 1) {
			// The caller belongs to two tenants that both registered this repo_key.
			// Picking one would file real metrics under an arbitrary governance
			// boundary, so it is refused BY NAME — and as a per-row rejection, so one
			// ambiguous repository cannot take the whole batch down with it.
			reject('ambiguous');
			continue;
		}
		const { id: repositoryId, tenant_id: tenantId } = ours[0];

		// From the batch map. `dojo.metric_catalogue` is a view over `sensei.metrics`,
		// because the `sensei` schema is deliberately NOT exposed to PostgREST (its
		// daemon tables have RLS disabled). Both shortcuts were tried live and both
		// failed as designed: `.from('metrics')` → "Could not find the table
		// 'dojo.metrics' in the schema cache", and `.schema('sensei')` → "Invalid
		// schema: sensei". A dojo view qualifying sensei.* internally is the
		// sanctioned route, same as dojo.rule_pack_library.
		const metricId = metricIds.get(row.metric);
		if (!metricId) {
			reject('unknown_metric');
			continue;
		}

		const values = {
			tenant_id: tenantId,
			repository_id: repositoryId,
			metric_id: metricId,
			scope: 'repo',
			// Explicitly null for a repo-scoped row: the value belongs to the
			// repository, not to a person.
			principal_id: null,
			commit_sha: row.commit_sha ?? null,
			computed_on: row.computed_on,
			grain: row.grain,
			value: row.value,
			props: row.props ?? {},
			source: row.source ?? 'measured'
		};

		// ALL SEVEN columns of the destination unique key — not the four this used to
		// match. `dojo.repository_metrics` is unique on (metric_id, repository_id,
		// scope, principal_id, commit_sha, computed_on, grain), and `quality.rs`
		// writes one repo-scoped row PER COMMIT per day. On the loose key those rows
		// all matched each other, so row 2 UPDATEd row 1 while both were counted
		// `accepted` — and the daemon then marked both `shared_at`, so the
		// overwritten value was gone and never re-sent. 6 groups / 34 rows in the
		// live database would have been destroyed the moment a second repository was
		// shared.
		//
		// `.is()` not `.eq()` for the nullable columns: PostgREST renders
		// `.eq(col, null)` as `col=eq.null`, which matches NOTHING in SQL — so an
		// `.eq` here would make every null-bearing row look new and insert a
		// duplicate on every cycle.
		let q = db
			.from('repository_metrics')
			.select('id')
			.eq('metric_id', metricId)
			.eq('repository_id', repositoryId)
			.eq('scope', 'repo')
			.is('principal_id', null)
			.eq('computed_on', row.computed_on)
			.eq('grain', row.grain);
		q = row.commit_sha == null ? q.is('commit_sha', null) : q.eq('commit_sha', row.commit_sha);
		const existing = await q.maybeSingle();
		if (existing.error) throw new AdminError(500, existing.error.message);

		const written = existing.data
			? await db
					.from('repository_metrics')
					.update({ ...values, pushed_at: new Date().toISOString() })
					.eq('id', (existing.data as { id: string }).id)
			: await db.from('repository_metrics').insert(values);
		if (written.error) throw new AdminError(500, written.error.message);
		accepted += 1;
	}

	return { accepted, rejected };
}
