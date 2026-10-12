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
import { refusalStatus } from './db-refusal';
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
 * File a batch of the caller's repo-scoped metric rows (daemon-sync.md §7), as
 * them (#236).
 *
 * This side RESOLVES: each row to one of the caller's repositories (their own
 * verdict rows, from dojo.my_repositories()) and to a catalogue metric, and
 * rejects by name what it can. The database WRITES: dojo.upsert_repository_metrics
 * re-checks every row against the caller's own verdict, takes the tenant from
 * it, and upserts on the seven-column key — one call for the batch.
 *
 * **`scope='user'` is refused, loudly.** `dojo.repository_metrics.principal_id`
 * is a PRINCIPAL, never a git email — commit trailers are unverified, so
 * accepting one would let anyone attribute work to a colleague. The bridge is
 * `sensei.personas.principal_id`, which is NULL until a persona is linked, so
 * there is currently nothing honest to attribute a user-scoped row to. Rejecting
 * names the gap; silently skipping would report a successful push that quietly
 * moved thousands of rows fewer than the daemon offered.
 */
export async function ingestMetrics(db: DojoClient, rows: MetricInput[]): Promise<IngestResult> {
	const rejected: IngestResult['rejected'] = [];
	if (rows.length === 0) return { accepted: 0, rejected };

	// BATCHED, not per row. This loop once issued ~4 PostgREST subrequests per row
	// inside ONE Cloudflare Worker invocation: at the 500 rows the daemon sends
	// that is ~2001 sequential round trips, past Cloudflare's per-invocation
	// subrequest cap and past the daemon's 30s timeout. Now: one verdict read, one
	// catalogue read, one write.
	const wantedKeys = [...new Set(rows.map((r) => r.repo_key))];
	const wantedMetrics = [...new Set(rows.map((r) => r.metric))];

	// THE GATE's inputs — the caller's own verdict rows, pinned to them in the
	// database. A REVOKED membership yields none (the view drops a disabled
	// membership), so an ex-member's daemon resolves nothing here and could write
	// nothing there.
	//
	// Scoped to THIS BATCH, not to the account. PostgREST caps an unbounded read
	// at `max_rows` (1000, verified live) and says nothing about truncating — so
	// reading every repository the caller can see would, past that ceiling,
	// silently refuse a repository the user IS entitled to sync, with
	// `not_permitted` naming the wrong reason. A batch is capped at MAX_ROWS
	// metrics, so `wantedKeys` cannot exceed it.
	const mineRead = await db
		.rpc('my_repositories')
		.select('repository_id, tenant_id, repo_key, sync_enabled')
		.in('repo_key', wantedKeys);
	if (mineRead.error) throw new AdminError(500, mineRead.error.message);
	const byKey = new Map<string, { repository_id: string; sync_enabled: boolean }[]>();
	for (const r of (mineRead.data ?? []) as { repository_id: string; repo_key: string; sync_enabled: boolean }[]) {
		byKey.set(r.repo_key, [...(byKey.get(r.repo_key) ?? []), r]);
	}

	// `dojo.metric_catalogue` is a view over `sensei.metrics`, because the `sensei`
	// schema is deliberately NOT exposed to PostgREST (its daemon tables have RLS
	// disabled). Both shortcuts were tried live and both failed as designed:
	// `.from('metrics')` → "Could not find the table 'dojo.metrics' in the schema
	// cache", and `.schema('sensei')` → "Invalid schema: sensei".
	const metricRows = await db.from('metric_catalogue').select('id, key').in('key', wantedMetrics);
	if (metricRows.error) throw new AdminError(500, metricRows.error.message);
	const metricIds = new Map(((metricRows.data ?? []) as { id: string; key: string }[]).map((m) => [m.key, m.id]));

	const toWrite: Record<string, unknown>[] = [];
	for (const row of rows) {
		const reject = (reason: RejectReason) => rejected.push({ repo_key: row.repo_key, metric: row.metric, reason });

		if (row.scope !== 'repo') {
			reject('unsupported_scope');
			continue;
		}
		// Not among the caller's repositories: never registered, or another
		// tenant's. Deliberately ONE answer — distinguishing them is an EXISTENCE
		// ORACLE: signup is open, so any stranger could post up to MAX_ROWS probe
		// rows per request and learn which private repo_keys other organisations
		// have enrolled.
		const ours = byKey.get(row.repo_key) ?? [];
		if (ours.length === 0) {
			reject('unknown_repository');
			continue;
		}
		if (!ours.some((r) => r.sync_enabled)) {
			reject('not_permitted');
			continue;
		}
		if (ours.length > 1) {
			// The caller belongs to two tenants that both registered this repo_key
			// ("a consultant legitimately has the same repository under two
			// clients"). Picking one would file real metrics under an arbitrary
			// governance boundary, so it is refused BY NAME, per row.
			reject('ambiguous');
			continue;
		}
		const metricId = metricIds.get(row.metric);
		if (!metricId) {
			reject('unknown_metric');
			continue;
		}
		// No tenant: the database takes it from the caller's verdict row.
		toWrite.push({
			repository_id: ours[0].repository_id,
			metric_id: metricId,
			commit_sha: row.commit_sha ?? null,
			computed_on: row.computed_on,
			grain: row.grain,
			value: row.value,
			props: row.props ?? {},
			source: row.source ?? 'measured'
		});
	}

	if (toWrite.length > 0) {
		const { error } = await db.rpc('upsert_repository_metrics', { p_rows: toWrite });
		if (error) throw new AdminError(refusalStatus(error), error.message);
	}
	return { accepted: toWrite.length, rejected };
}
