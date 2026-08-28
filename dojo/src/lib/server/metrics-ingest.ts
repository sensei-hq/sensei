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
	| 'not_permitted' //     the caller is not a member of the repo's tenant
	| 'unknown_repository' // no registered repository has that repo_key
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
	const mem = await db.from('memberships').select('tenant_id').eq('user_id', principalId);
	if (mem.error) throw new AdminError(500, mem.error.message);
	const mine = new Set(
		((mem.data ?? []) as { tenant_id: string }[]).map((m) => m.tenant_id)
	);

	for (const row of rows) {
		const reject = (reason: RejectReason) =>
			rejected.push({ repo_key: row.repo_key, metric: row.metric, reason });

		if (row.scope !== 'repo') {
			reject('unsupported_scope');
			continue;
		}

		// Scoped to the CALLER'S TENANTS, and a plain `.select()` — never
		// `.maybeSingle()`. `dojo.repositories` is `unique (tenant_id, repo_key)`
		// and its DDL says so deliberately: "ONE ROW PER (repo_key, tenant), not one
		// globally. A consultant legitimately has the same repository under two
		// clients." An unscoped `.maybeSingle()` therefore returned PGRST116 the
		// moment that happened, which threw a 500 and killed the ENTIRE batch —
		// permanently, because nothing gets marked shared and the identical batch
		// retries every cadence. `registerRepositories` was already tenant-scoped;
		// this half was not, and the two disagreeing is what made it reachable.
		const repos = await db
			.from('repositories')
			.select('id, tenant_id')
			.eq('repo_key', row.repo_key);
		if (repos.error) throw new AdminError(500, repos.error.message);
		const all = (repos.data ?? []) as { id: string; tenant_id: string }[];
		if (all.length === 0) {
			reject('unknown_repository');
			continue;
		}
		const ours = all.filter((r) => mine.has(r.tenant_id));
		if (ours.length === 0) {
			// Not `unknown_repository`: it exists, the caller just has no business
			// writing to it. Conflating the two would also leak whether a given
			// repo_key is registered in somebody else's tenant.
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

		// `dojo.metric_catalogue` — a view over `sensei.metrics`, because the
		// `sensei` schema is deliberately NOT exposed to PostgREST (its daemon
		// tables have RLS disabled). Both shortcuts were tried live and both
		// failed as designed: `.from('metrics')` → "Could not find the table
		// 'dojo.metrics' in the schema cache", and `.schema('sensei')` →
		// "Invalid schema: sensei". A dojo view qualifying sensei.* internally is
		// the sanctioned route, same as dojo.rule_pack_library.
		const metric = await db
			.from('metric_catalogue')
			.select('id')
			.eq('key', row.metric)
			.maybeSingle();
		if (metric.error) throw new AdminError(500, metric.error.message);
		if (!metric.data) {
			reject('unknown_metric');
			continue;
		}
		const metricId = (metric.data as { id: string }).id;

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
