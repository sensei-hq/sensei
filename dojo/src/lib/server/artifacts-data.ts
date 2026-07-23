// Pure/injectable data logic for the Worker artifact-federation endpoints
// (`/v1/t/{origin}/{org}/artifacts`) — the TS port of dojo-mind's
// `DojoStore::publish_artifact` / `pull_artifacts_since` (store.rs) plus the
// INLINE `promote_cluster` collective-promotion stage
// (`crates/dojo-mind/src/collective/promote.rs`). Kept out of `+server.ts` so the
// k-anonymity / clustering / score math and the wire-shape logic are
// unit-testable without a Worker (mirrors `rules-data.ts`).
//
// Wire contract: matches `dojo_protocol` (`crates/dojo-protocol/src/lib.rs`)
// byte-for-byte so senseid's dojo client (`crates/senseid/src/dojo/client.rs`,
// `publish_artifact` ~:174 / `pull_artifacts` ~:206) retargets with only a
// URL/auth swap:
//   • publish returns `PublishArtifactResponse { id, seq }` (no version — artifacts
//     have no version column, unlike rules).
//   • pull returns `ArtifactPullResponse { artifacts: PulledArtifact[], cursor }`,
//     where a `PulledArtifact` is `{ id, seq, status, ...PublishedArtifact }`
//     (`#[serde(flatten)]` — the artifact fields sit at the top level).
//
// The daemon publishes at `status='submitted'`; the collective-promote stage
// moves clusters `submitted → published` (auto-approve) or queues them for a
// maintainer, so C7's pull (`status='published'`) can distribute them. The pure
// decision logic (`ClusterSignals` / `score` / `decide` / the k-anonymity gate)
// is ported verbatim from `promote.rs` and unit-tested with no DB.
//
// PostgREST divergence from the Rust store (documented, tracked): dojo-mind runs
// publish + promote_cluster each in ONE Postgres transaction holding
// `pg_advisory_xact_lock(ARTIFACTS_SEQ_LOCK)`, and advances `seq` with
// `nextval('dojo.artifacts_seq')` inline. supabase-js/PostgREST can neither open
// a multi-statement transaction, take an advisory lock, nor call `nextval`
// inline. So:
//   • seq is assigned by the `dojo.artifacts.seq` column DEFAULT
//     (`nextval('dojo.artifacts_seq')`) on INSERT — matching dojo-mind's
//     first-write seq exactly. Promotion (submitted→published) keeps that
//     insert-time seq rather than re-stamping it (the SAME divergence the rules
//     endpoint documents for republish/retract); advancing seq on the UPDATE
//     path needs an in-DB trigger (precedent: `dojo.relay_inbox_seq_bump`) — a
//     shared DDL follow-up for both federation endpoints. In practice the
//     published row still rides a puller's delta at its insert seq (pull returns
//     all statuses with `seq > since`); see `publishRepresentative`.
//   • the promote step is a sequence of PostgREST calls rather than one txn. It
//     is still idempotent (guards on artifact `status` / triage `state` exactly
//     as dojo-mind does), so a re-run neither double-publishes nor double-decides.
//     The narrow window between calls is acceptable: promotion runs single-writer
//     off the publish path and a re-run is safe.
// The pure k-anonymity / cluster math matches dojo-mind bit-for-bit.

import type { dojoDb } from './dojo-supabase';

/** The supabase-js client returned by `dojoDb()` (scoped to the `dojo` schema). */
export type DojoClient = ReturnType<typeof dojoDb>;

// ── dojo_protocol wire types (artifact contract) ─────────────────────────────

/** `dojo_protocol::ArtifactKind` — the six primitives (`dojo.artifact_kind`). */
export const ARTIFACT_KINDS = [
	'principle',
	'pattern',
	'prompt',
	'guard',
	'skill',
	'agent'
] as const;
export type ArtifactKind = (typeof ARTIFACT_KINDS)[number];

/** `dojo_protocol::ArtifactStatus` (`dojo.artifact_status`). */
export type ArtifactStatus = 'submitted' | 'published' | 'archived';

/** `dojo_protocol::Attribution` — the `dojo.artifacts.attribution` jsonb. */
export interface Attribution {
	mode: 'named' | 'anonymous' | 'dereferenced';
	author?: string | null;
	org?: string | null;
	anonymous_id?: string | null;
	dereferenced: boolean;
}

/** `dojo_protocol::ArtifactScope` — the `dojo.artifacts.scope` jsonb. */
export interface ArtifactScope {
	company?: string | null;
	team?: string | null;
	project?: string | null;
	stack?: string | null;
}

/** `dojo_protocol::PublishedArtifact` — the flattened contribution snapshot. */
export interface PublishedArtifact {
	signature: string;
	tenant_key: string;
	engagement_id?: string | null;
	kind: ArtifactKind;
	title: string;
	body: string;
	/** Type-specific jsonb payload, internally tagged on `kind`. */
	payload: Record<string, unknown>;
	scope: ArtifactScope;
	attribution: Attribution;
	dereferenced: boolean;
	/** Client-supplied but IGNORED — attribution (`contributed_by`) is server-controlled. */
	contributed_by?: string | null;
	/** Client-supplied but IGNORED — stamped server-side on publish. */
	published_at?: string | null;
}

/** `dojo_protocol::PublishArtifactResponse` (no `version` — artifacts have none). */
export interface PublishArtifactResponse {
	id: string;
	seq: number;
}

/** `dojo_protocol::PulledArtifact` — the flattened `PublishedArtifact` plus identity. */
export interface PulledArtifact extends PublishedArtifact {
	id: string;
	seq: number;
	status: ArtifactStatus;
}

/** `dojo_protocol::ArtifactPullResponse`. */
export interface ArtifactPullResponse {
	artifacts: PulledArtifact[];
	cursor: number;
}

/** A domain error carrying the HTTP status the handler should return. */
export class ArtifactError extends Error {
	constructor(
		readonly status: number,
		message: string
	) {
		super(message);
	}
}

// ── Pure triage logic (ported verbatim from promote.rs; unit-tested, no DB) ───

/**
 * Cluster score at or above which a cluster auto-publishes; below it, the
 * cluster is queued for a maintainer. Chosen so five independent contributors
 * alone clear it, while a single strong contribution can also clear it on
 * measured efficacy. (`promote.rs::AUTO_APPROVE_SCORE`.)
 */
export const AUTO_APPROVE_SCORE = 0.8;

/**
 * Weight each distinct contributor adds to the score (breadth signal). Five
 * contributors → `5 * 0.16 = 0.80` = exactly the bar.
 * (`promote.rs::PER_CONTRIBUTOR_WEIGHT`.)
 */
export const PER_CONTRIBUTOR_WEIGHT = 0.16;

/**
 * Observed first-try-resolution delta that on its own earns full efficacy
 * credit (1.0). (`promote.rs::FTR_DELTA_FULL_CREDIT`.)
 */
export const FTR_DELTA_FULL_CREDIT = 0.2;

/**
 * k-anonymity floor: a `global`-scope (global-dojo) artifact may auto-publish
 * only once at least this many DISTINCT contributors stand behind it — never
 * publish a global artifact that could de-anonymise a lone contributor.
 * (`promote.rs::K_ANONYMITY_MIN_CONTRIBUTORS`.)
 */
export const K_ANONYMITY_MIN_CONTRIBUTORS = 3;

/**
 * The confidence signals aggregated over one signature-cluster of `submitted`
 * artifacts — the pure input to `score` / `decide` (`promote.rs::ClusterSignals`).
 */
export interface ClusterSignals {
	/** Distinct contributors behind the cluster (drives breadth + k-anonymity). */
	contributorCount: number;
	/** Best observed FTR lift across the cluster's pattern payloads, if any. */
	ftrDeltaObserved: number | null;
	/** True when the owning tenant is the `global` collective (k-anonymity applies). */
	isGlobal: boolean;
}

/**
 * Clamp `n` into `[lo, hi]`. Rust's `f64::clamp` (used by `score`) treats a NaN
 * input by returning it, but our aggregate never produces NaN; a plain clamp
 * matches the observed behaviour for every finite input.
 */
function clamp(n: number, lo: number, hi: number): number {
	return Math.min(Math.max(n, lo), hi);
}

/**
 * Confidence score in `0.0..=1.0` from a cluster's signals. Two additive,
 * each-saturating evidence sources (verbatim from `promote.rs::score`):
 *   • breadth  = `max(0, contributorCount) * PER_CONTRIBUTOR_WEIGHT`
 *   • efficacy = `max(0, ftrDeltaObserved) / FTR_DELTA_FULL_CREDIT`
 * The sum is clamped to 1.0; a negative/absent FTR delta adds no efficacy and
 * never penalises breadth; a below-zero contributor count can't go negative.
 */
export function score(s: ClusterSignals): number {
	const breadth = Math.max(s.contributorCount, 0) * PER_CONTRIBUTOR_WEIGHT;
	const efficacy = Math.max(s.ftrDeltaObserved ?? 0, 0) / FTR_DELTA_FULL_CREDIT;
	return clamp(breadth + efficacy, 0, 1);
}

/** Why a cluster was queued instead of auto-approved (`promote.rs::QueueReason`). */
export type QueueReason = 'below_score_bar' | 'k_anonymity';

/** The pure triage verdict for a cluster (`promote.rs::Decision`). */
export type Decision =
	| { kind: 'auto_approve' }
	| { kind: 'queue'; reason: QueueReason };

/**
 * Decide a cluster's fate from its signals (verbatim from `promote.rs::decide`):
 * the score must clear `AUTO_APPROVE_SCORE`, and a `global` cluster must
 * additionally clear the `K_ANONYMITY_MIN_CONTRIBUTORS` floor. The score bar is
 * checked FIRST so a low-confidence global cluster reports the more informative
 * `below_score_bar`.
 */
export function decide(s: ClusterSignals): Decision {
	if (score(s) < AUTO_APPROVE_SCORE) {
		return { kind: 'queue', reason: 'below_score_bar' };
	}
	if (s.isGlobal && s.contributorCount < K_ANONYMITY_MIN_CONTRIBUTORS) {
		return { kind: 'queue', reason: 'k_anonymity' };
	}
	return { kind: 'auto_approve' };
}

/** Outcome of promoting one signature-cluster (`promote.rs::PromoteOutcome`). */
export type PromoteOutcome =
	| { outcome: 'published'; artifact_id: string; seq: number }
	| { outcome: 'merged'; into: string }
	| { outcome: 'queued'; reason: QueueReason }
	| { outcome: 'no_op' };

// ── Publish-body validation (mirrors parsePublishedRule) ─────────────────────

/** The kind → payload-tag map: `payload.kind` MUST equal the envelope `kind`. */
function kindMatchesPayload(kind: unknown, payload: unknown): boolean {
	if (!ARTIFACT_KINDS.includes(kind as ArtifactKind)) return false;
	if (typeof payload !== 'object' || payload === null) return false;
	return (payload as Record<string, unknown>).kind === kind;
}

/**
 * Validate + normalize an untrusted publish body into a `PublishedArtifact`, or
 * return `null` when a required field is missing/blank or the payload's `kind`
 * tag doesn't match the envelope `kind` (the handler maps that to 400 — mirrors
 * dojo-mind's `kind_matches_payload` guard). Optional fields coerce to null.
 * `contributed_by` / `published_at` from the body are intentionally NOT read —
 * the handler stamps `contributed_by` from the authenticated caller and the
 * store stamps `published_at` on promotion (attribution is server-controlled).
 */
export function parsePublishedArtifact(body: Record<string, unknown>): PublishedArtifact | null {
	const nonEmptyStr = (v: unknown): v is string => typeof v === 'string' && v.trim() !== '';
	for (const f of ['signature', 'tenant_key', 'title', 'body'] as const) {
		if (!nonEmptyStr(body[f])) return null;
	}
	if (!kindMatchesPayload(body.kind, body.payload)) return null;

	const attribution = body.attribution;
	if (typeof attribution !== 'object' || attribution === null) return null;
	const attr = attribution as Record<string, unknown>;
	if (attr.mode !== 'named' && attr.mode !== 'anonymous' && attr.mode !== 'dereferenced') {
		return null;
	}
	if (typeof attr.dereferenced !== 'boolean') return null;

	const optStr = (v: unknown): string | null => (nonEmptyStr(v) ? v : null);
	const scope = (typeof body.scope === 'object' && body.scope !== null ? body.scope : {}) as ArtifactScope;

	return {
		signature: body.signature as string,
		tenant_key: body.tenant_key as string,
		engagement_id: optStr(body.engagement_id),
		kind: body.kind as ArtifactKind,
		title: body.title as string,
		body: body.body as string,
		payload: body.payload as Record<string, unknown>,
		scope,
		attribution: {
			mode: attr.mode,
			author: optStr(attr.author),
			org: optStr(attr.org),
			anonymous_id: optStr(attr.anonymous_id),
			dereferenced: attr.dereferenced
		},
		dereferenced: body.dereferenced === true
	};
}

// ── Store: publish (mirrors DojoStore::publish_artifact) ──────────────────────

/**
 * Publish (contribute) an artifact under `tenantId`: insert into `dojo.artifacts`
 * at the DDL-default `status='submitted'` (pending triage), stamping
 * `contributed_by` from the authenticated caller (never the body). `seq` is
 * assigned by the column DEFAULT (`nextval('dojo.artifacts_seq')`) on INSERT —
 * matching dojo-mind's first-write seq. Returns `PublishArtifactResponse
 * { id, seq }`.
 *
 * dojo-mind dedups by relying on triage to merge same-signature re-submissions
 * (`promote_cluster`'s already-published → merge path); publish itself always
 * inserts, so this mirrors it (no upsert).
 */
export async function publishArtifact(
	db: DojoClient,
	tenantId: string,
	artifact: PublishedArtifact,
	contributedBy: string | null
): Promise<PublishArtifactResponse> {
	const { data, error } = await db
		.from('artifacts')
		.insert({
			tenant_id: tenantId,
			engagement_id: artifact.engagement_id,
			kind: artifact.kind,
			title: artifact.title,
			body: artifact.body,
			payload: artifact.payload,
			scope: artifact.scope,
			signature: artifact.signature,
			attribution: artifact.attribution,
			dereferenced: artifact.dereferenced,
			contributed_by: contributedBy
		})
		.select('id, seq')
		.single();
	if (error) throw new ArtifactError(500, error.message);
	const row = data as { id: string; seq: number };
	return { id: row.id, seq: row.seq };
}

// ── Store: pull (mirrors DojoStore::pull_artifacts_since) ─────────────────────

/**
 * One joined `dojo.artifacts ⋈ dojo.tenants` row as returned by the pull select.
 * The fields come off PostgREST loosely typed (`status`/`kind` are DB text), so
 * they widen to `string`; `shapeArtifactPullResponse` re-narrows them onto the
 * `PulledArtifact` wire types. The embedded tenant is a to-one FK-embed, so
 * supabase-js returns an object (or, defensively, a one-element array).
 */
export interface ArtifactRow {
	id: string;
	seq: number;
	status: string;
	engagement_id: string | null;
	kind: string;
	title: string;
	body: string;
	payload: Record<string, unknown>;
	scope: ArtifactScope;
	signature: string;
	attribution: Attribution;
	dereferenced: boolean;
	contributed_by: string | null;
	published_at: string | null;
	tenants: { key: string } | { key: string }[] | null;
}

/** The `select` list for the pull join (artifact columns + embedded tenant key). */
export const ARTIFACT_PULL_SELECT =
	'id, seq, status, engagement_id, kind, title, body, payload, scope, signature, attribution, dereferenced, contributed_by, published_at, tenants(key)';

/** Normalize PostgREST's embed (object or one-element array) to the single row. */
function embeddedTenant(t: ArtifactRow['tenants']): { key: string } | null {
	if (!t) return null;
	return Array.isArray(t) ? (t[0] ?? null) : t;
}

/**
 * Shape a set of joined rows into an `ArtifactPullResponse` and compute the
 * cursor as `max(seq)` (or `since` when the page is empty) — mirrors
 * `DojoStore::pull_artifacts_since`. Pure over its input so the wire-shape /
 * cursor math is unit-tested directly.
 */
export function shapeArtifactPullResponse(rows: ArtifactRow[], since: number): ArtifactPullResponse {
	let cursor = since;
	const artifacts: PulledArtifact[] = rows.map((r) => {
		if (r.seq > cursor) cursor = r.seq;
		const t = embeddedTenant(r.tenants);
		return {
			id: r.id,
			seq: r.seq,
			// PostgREST returns these as DB text; re-narrow onto the wire enums.
			status: r.status as ArtifactStatus,
			signature: r.signature,
			tenant_key: t?.key ?? '',
			engagement_id: r.engagement_id,
			kind: r.kind as ArtifactKind,
			title: r.title,
			body: r.body,
			payload: r.payload,
			scope: r.scope,
			attribution: r.attribution,
			dereferenced: r.dereferenced,
			contributed_by: r.contributed_by,
			published_at: r.published_at
		};
	});
	return { artifacts, cursor };
}

/**
 * Pull a tenant's artifacts with `seq > since`, ordered by `seq` — the query
 * half of `DojoStore::pull_artifacts_since`. Strictly scoped to `tenantId`, so
 * one tenant never sees another's artifacts. Delegates shaping to
 * `shapeArtifactPullResponse`.
 */
export async function pullArtifactsSince(
	db: DojoClient,
	tenantId: string,
	since: number
): Promise<ArtifactPullResponse> {
	const { data, error } = await db
		.from('artifacts')
		.select(ARTIFACT_PULL_SELECT)
		.eq('tenant_id', tenantId)
		.gt('seq', since)
		.order('seq', { ascending: true });
	if (error) throw new ArtifactError(500, error.message);
	return shapeArtifactPullResponse((data ?? []) as unknown as ArtifactRow[], since);
}

// ── Store: promote_cluster (inline; mirrors promote.rs::promote_cluster) ──────

/** One `submitted` cluster member as loaded for the aggregate. */
interface ClusterMember {
	id: string;
	seq: number;
	scope: ArtifactScope;
	contributed_by: string | null;
	attribution: Attribution | null;
	payload: Record<string, unknown> | null;
}

/**
 * Distinct-contributor tally for a cluster — `count(distinct
 * coalesce(contributed_by, attribution->>'anonymous_id'))` from promote.rs. Pure
 * over the loaded members so the tally is unit-tested directly. A member with
 * neither a `contributed_by` nor an `anonymous_id` has a NULL coalesce and so
 * contributes nothing to the tally — matching Postgres `count(distinct)`, which
 * ignores NULLs.
 */
export function distinctContributors(members: ClusterMember[]): number {
	const ids = new Set<string>();
	for (const m of members) {
		const id = m.contributed_by ?? m.attribution?.anonymous_id ?? null;
		if (id != null) ids.add(id);
	}
	return ids.size;
}

/**
 * Best observed FTR lift across the cluster's pattern payloads — `max((payload->>
 * 'ftr_delta_observed')::float8)` from promote.rs. Pure. `null` when no member
 * carries one.
 */
export function bestFtrDelta(members: ClusterMember[]): number | null {
	let best: number | null = null;
	for (const m of members) {
		const raw = m.payload?.ftr_delta_observed;
		const n = typeof raw === 'number' ? raw : typeof raw === 'string' ? Number.parseFloat(raw) : NaN;
		if (Number.isFinite(n)) best = best == null ? n : Math.max(best, n);
	}
	return best;
}

/** Load the tenant's `scope = 'global'` flag (k-anonymity gate applicability). */
async function tenantIsGlobal(db: DojoClient, tenantId: string): Promise<boolean | null> {
	const { data, error } = await db.from('tenants').select('scope').eq('id', tenantId).maybeSingle();
	if (error) throw new ArtifactError(500, error.message);
	if (!data) return null;
	return (data as { scope: string }).scope === 'global';
}

/** Load a signature-cluster's `submitted` members (ordered by seq → representative first). */
async function loadSubmittedCluster(
	db: DojoClient,
	tenantId: string,
	signature: string
): Promise<ClusterMember[]> {
	const { data, error } = await db
		.from('artifacts')
		.select('id, seq, scope, contributed_by, attribution, payload')
		.eq('tenant_id', tenantId)
		.eq('signature', signature)
		.eq('status', 'submitted')
		.order('seq', { ascending: true });
	if (error) throw new ArtifactError(500, error.message);
	return (data ?? []) as unknown as ClusterMember[];
}

/** The id of an already-`published` artifact for this signature, or null. */
async function alreadyPublishedId(
	db: DojoClient,
	tenantId: string,
	signature: string
): Promise<string | null> {
	const { data, error } = await db
		.from('artifacts')
		.select('id')
		.eq('tenant_id', tenantId)
		.eq('signature', signature)
		.eq('status', 'published')
		.order('seq', { ascending: true })
		.limit(1)
		.maybeSingle();
	if (error) throw new ArtifactError(500, error.message);
	return data ? (data as { id: string }).id : null;
}

/**
 * Publish the representative artifact (`status → 'published'`, stamp
 * `published_at`), guarding on `status='submitted'` so a re-run can't
 * re-publish. Returns the row's seq, or null when nothing was in `submitted`
 * (an idempotent re-run, or a concurrent promote already won the row).
 *
 * Seq on publish: dojo-mind re-stamps `seq = nextval('dojo.artifacts_seq')`
 * inside the promote transaction so the published row re-surfaces in a puller's
 * delta. PostgREST can't call `nextval` inline on an UPDATE, so this keeps the
 * insert-time seq — the SAME divergence the rules endpoint documents
 * (`rules-data.ts::publishRule`). In practice the published row still rides the
 * puller's delta at its insert seq (the pull returns all statuses with `seq >
 * since`), unless a puller had already advanced past that seq while the artifact
 * was still `submitted`. Advancing seq on UPDATE needs an in-DB trigger
 * (precedent: `dojo.relay_inbox_seq_bump`) — a shared DDL follow-up for both
 * federation endpoints, not a per-endpoint workaround.
 */
async function publishRepresentative(db: DojoClient, repId: string): Promise<number | null> {
	const { data, error } = await db
		.from('artifacts')
		.update({ status: 'published', published_at: new Date().toISOString(), updated_at: new Date().toISOString() })
		.eq('id', repId)
		.eq('status', 'submitted')
		.select('seq')
		.maybeSingle();
	if (error) throw new ArtifactError(500, error.message);
	return data ? (data as { seq: number }).seq : null;
}

/**
 * Archive the cluster's `submitted` artifacts (optionally sparing `except` — the
 * just-published representative). Merged duplicates were never distributed, so
 * `seq` is left untouched. Mirrors promote.rs::archive_submitted_cluster.
 */
async function archiveSubmittedCluster(
	db: DojoClient,
	tenantId: string,
	signature: string,
	except: string | null
): Promise<void> {
	let q = db
		.from('artifacts')
		.update({ status: 'archived', updated_at: new Date().toISOString() })
		.eq('tenant_id', tenantId)
		.eq('signature', signature)
		.eq('status', 'submitted');
	if (except) q = q.neq('id', except);
	const { error } = await q;
	if (error) throw new ArtifactError(500, error.message);
}

/**
 * Upsert the cluster's single `dojo.triage_queue` row (keyed by `(tenant_id,
 * signature)`). Select-then-insert/update because the table has no unique
 * constraint on that pair (mirrors promote.rs::upsert_triage). Returns whether a
 * row was newly inserted (so the caller only logs `queued` on first insert).
 */
async function upsertTriage(
	db: DojoClient,
	tenantId: string,
	artifactId: string,
	signature: string,
	ownerScope: ArtifactScope,
	confidence: number,
	contributorCount: number,
	similarity: number | null,
	nearest: string | null,
	state: string
): Promise<{ triageId: string; inserted: boolean }> {
	const existing = await db
		.from('triage_queue')
		.select('id')
		.eq('tenant_id', tenantId)
		.eq('signature', signature)
		.limit(1)
		.maybeSingle();
	if (existing.error) throw new ArtifactError(500, existing.error.message);

	const fields = {
		artifact_id: artifactId,
		owner_scope: ownerScope,
		confidence,
		contributor_count: contributorCount,
		similarity,
		nearest_artifact_id: nearest,
		state,
		updated_at: new Date().toISOString()
	};

	if (existing.data) {
		const id = (existing.data as { id: string }).id;
		const { error } = await db.from('triage_queue').update(fields).eq('id', id);
		if (error) throw new ArtifactError(500, error.message);
		return { triageId: id, inserted: false };
	}
	const { data, error } = await db
		.from('triage_queue')
		.insert({ tenant_id: tenantId, signature, ...fields })
		.select('id')
		.single();
	if (error) throw new ArtifactError(500, error.message);
	return { triageId: (data as { id: string }).id, inserted: true };
}

/** Append a `dojo.decisions` row (the named triage verdict trail). */
async function insertDecision(
	db: DojoClient,
	tenantId: string,
	artifactId: string,
	triageId: string | null,
	maintainerId: string | null,
	status: 'approve' | 'revise' | 'decline',
	reason: string | null,
	distributionScope: ArtifactScope | null,
	automated: boolean
): Promise<void> {
	const { error } = await db.from('decisions').insert({
		tenant_id: tenantId,
		artifact_id: artifactId,
		triage_id: triageId,
		maintainer_id: maintainerId,
		status,
		reason,
		distribution_scope: distributionScope,
		automated
	});
	if (error) throw new ArtifactError(500, error.message);
}

/** Append a `dojo.events` row (the append-only operational log). */
async function insertEvent(
	db: DojoClient,
	tenantId: string,
	actorId: string | null,
	artifactId: string | null,
	action: string,
	detail: Record<string, unknown>
): Promise<void> {
	const { error } = await db.from('events').insert({
		tenant_id: tenantId,
		actor_id: actorId,
		artifact_id: artifactId,
		action,
		detail
	});
	if (error) throw new ArtifactError(500, error.message);
}

/**
 * Promote one signature-cluster within `tenantId` — the INLINE port of
 * dojo-mind's `promote_cluster`. Aggregates the `submitted` cluster (distinct
 * contributors, best FTR lift, representative = earliest by seq), then:
 *   • nothing submitted → `no_op` (idempotent — already resolved);
 *   • signature already published → archive the re-submissions (`merged`);
 *   • else `decide(signals)`: `auto_approve` publishes the representative +
 *     archives the rest + records an automated approval; `queue` upserts the
 *     triage row (logging `queued` only on first insert).
 *
 * The k-anonymity gate applies only when the tenant is `global`.
 *
 * Idempotent by the same status/state guards as dojo-mind. Not one Postgres
 * transaction (PostgREST can't) — see the module header for the documented
 * divergence and why it is safe on the single-writer promote path.
 */
export async function promoteCluster(
	db: DojoClient,
	tenantId: string,
	signature: string
): Promise<PromoteOutcome> {
	const isGlobal = await tenantIsGlobal(db, tenantId);
	if (isGlobal == null) {
		// Unknown tenant — nothing to promote (the handler validated existence).
		return { outcome: 'no_op' };
	}

	const members = await loadSubmittedCluster(db, tenantId, signature);
	if (members.length === 0) {
		// Already published/archived — re-running neither double-publishes nor
		// double-decides.
		return { outcome: 'no_op' };
	}
	const rep = members[0]; // earliest by seq = the representative
	const ownerScope = rep.scope ?? {};

	const contributorCount = distinctContributors(members);
	const signals: ClusterSignals = {
		contributorCount,
		ftrDeltaObserved: bestFtrDelta(members),
		isGlobal
	};
	const confidence = score(signals);

	// If the signature is already published, the candidates are re-submissions of
	// live content: merge (archive) them rather than publish a duplicate.
	const publishedInto = await alreadyPublishedId(db, tenantId, signature);
	if (publishedInto) {
		await archiveSubmittedCluster(db, tenantId, signature, null);
		await upsertTriage(
			db,
			tenantId,
			rep.id,
			signature,
			ownerScope,
			confidence,
			contributorCount,
			1.0,
			publishedInto,
			'merged'
		);
		await insertEvent(db, tenantId, null, publishedInto, 'merged', {
			signature,
			merged_into: publishedInto,
			candidates: members.length,
			automated: true
		});
		return { outcome: 'merged', into: publishedInto };
	}

	const decision = decide(signals);
	if (decision.kind === 'auto_approve') {
		const pubSeq = await publishRepresentative(db, rep.id);
		if (pubSeq == null) {
			// Representative was no longer submitted (a concurrent promote won) —
			// treat as resolved rather than publish a duplicate.
			return { outcome: 'no_op' };
		}
		await archiveSubmittedCluster(db, tenantId, signature, rep.id);
		const { triageId } = await upsertTriage(
			db,
			tenantId,
			rep.id,
			signature,
			ownerScope,
			confidence,
			contributorCount,
			null,
			null,
			'auto_approved'
		);
		await insertDecision(
			db,
			tenantId,
			rep.id,
			triageId,
			null,
			'approve',
			'auto-approved: cleared the collective trust bar',
			ownerScope,
			true
		);
		await insertEvent(db, tenantId, null, rep.id, 'approved', {
			automated: true,
			score: confidence,
			contributor_count: contributorCount
		});
		return { outcome: 'published', artifact_id: rep.id, seq: pubSeq };
	}

	// Queue for a maintainer (below the score bar, or k-anonymity block).
	const { inserted } = await upsertTriage(
		db,
		tenantId,
		rep.id,
		signature,
		ownerScope,
		confidence,
		contributorCount,
		null,
		null,
		'queued'
	);
	// Log 'queued' only when the row is newly created — a re-run on an
	// already-queued cluster must not spam the log (mirrors promote.rs).
	if (inserted) {
		await insertEvent(db, tenantId, null, rep.id, 'queued', {
			reason: decision.reason,
			score: confidence,
			contributor_count: contributorCount
		});
	}
	return { outcome: 'queued', reason: decision.reason };
}
