// Pure/injectable data logic for the Worker maintainer-triage endpoints
// (`GET /v1/t/{origin}/{org}/triage` + `POST …/triage/{signature}/decide`) — the
// TS port of dojo-mind's triage read + decide over `dojo.triage_queue` (+ the
// `dojo.decisions` write). Kept out of `+server.ts` so the query shaping and the
// decide-body validation are unit-testable without a Worker.
//
// Wire contract: matches the console client `triage-data.ts` (the shipped
// `(console)` maintainer screen) — `GET` returns `{ queue: TriageRow[] }`, the
// open rows only (`queued | in_review`), ranked strongest-first; `decide` writes
// a `dojo.decisions` row and flips `triage_queue.state`, returning the flattened
// `DecideResult { status, artifact_id?, seq? }`.

import type { dojoDb } from './dojo-supabase';

/** The supabase-js client returned by `dojoDb()` (scoped to the `dojo` schema). */
export type DojoClient = ReturnType<typeof dojoDb>;

/** A domain error carrying the HTTP status the handler should return. */
export class TriageError extends Error {
	constructor(
		readonly status: number,
		message: string
	) {
		super(message);
	}
}

/** One open triage row on the wire — the console `TriageRow` shape. */
export interface TriageRow {
	signature: string;
	artifact_id: string;
	kind: string;
	title: string;
	owner_scope: unknown;
	confidence: number | null;
	contributor_count: number;
	similarity: number | null;
	nearest_artifact_id: string | null;
	state: string;
	created_at: string;
}

/** The open triage states the console lists (`auto_approved | resolved | merged`
 *  are terminal and never surface in the queue). */
const OPEN_STATES = ['queued', 'in_review'] as const;

/** A maintainer verdict — the console `DecideStatus`. */
export type DecideStatus = 'approve' | 'revise' | 'decline';

/** A validated decide request. */
export interface DecideRequest {
	status: DecideStatus;
	distribution_scope?: unknown;
	reason?: string;
}

/** `POST …/decide` result — the flattened `DecideOutcome`. `artifact_id`/`seq`
 *  are present only on `approved`. */
export interface DecideResult {
	status: 'approved' | 'declined' | 'revised';
	artifact_id?: string;
	seq?: number;
}

// ── the artifact row the triage select embeds (kind + title live on the
//    artifact, not the queue row — mirrors dojo-mind's join). ──────────────────
interface TriageQueueRow {
	signature: string;
	artifact_id: string;
	owner_scope: unknown;
	confidence: number | null;
	contributor_count: number;
	similarity: number | null;
	nearest_artifact_id: string | null;
	state: string;
	created_at: string;
	// PostgREST FK-embed of `dojo.artifacts` (to-one → object, defensively an
	// array too).
	artifacts:
		| { kind: string; title: string }
		| { kind: string; title: string }[]
		| null;
}

/** The `select` list for the triage read — queue columns + the embedded artifact
 *  (kind + title). */
export const TRIAGE_SELECT =
	'signature, artifact_id, owner_scope, confidence, contributor_count, similarity, nearest_artifact_id, state, created_at, artifacts(kind, title)';

/** Normalize PostgREST's embed (object or one-element array) to the single row. */
function embeddedArtifact(
	a: TriageQueueRow['artifacts']
): { kind: string; title: string } | null {
	if (!a) return null;
	return Array.isArray(a) ? (a[0] ?? null) : a;
}

/**
 * Rank open triage rows strongest-first: highest confidence, then newest; a
 * null confidence sorts last. Pure over its input (mirrors the console's
 * `groupByScope` intra-group order but flat, since the console re-groups by
 * scope client-side). Same input → same output.
 */
export function rankTriageRows(rows: TriageRow[]): TriageRow[] {
	return [...rows].sort((a, b) => {
		const ca = a.confidence ?? -1;
		const cb = b.confidence ?? -1;
		if (cb !== ca) return cb - ca;
		return b.created_at.localeCompare(a.created_at);
	});
}

/** Shape the joined queue rows into the flat `TriageRow[]` wire list. Pure. */
export function shapeTriageRows(rows: TriageQueueRow[]): TriageRow[] {
	const shaped = rows.map((r) => {
		const art = embeddedArtifact(r.artifacts);
		return {
			signature: r.signature,
			artifact_id: r.artifact_id,
			kind: art?.kind ?? '',
			title: art?.title ?? '',
			owner_scope: r.owner_scope,
			confidence: r.confidence,
			contributor_count: r.contributor_count,
			similarity: r.similarity,
			nearest_artifact_id: r.nearest_artifact_id,
			state: r.state,
			created_at: r.created_at
		};
	});
	return rankTriageRows(shaped);
}

/**
 * List the tenant's OPEN triage rows (`queued | in_review`), joined to their
 * artifact for kind + title, ranked strongest-first. The port of dojo-mind's
 * triage list.
 */
export async function listTriage(db: DojoClient, tenantId: string): Promise<TriageRow[]> {
	const { data, error } = await db
		.from('triage_queue')
		.select(TRIAGE_SELECT)
		.eq('tenant_id', tenantId)
		.in('state', OPEN_STATES as unknown as string[])
		.order('created_at', { ascending: false });
	if (error) throw new TriageError(500, error.message);
	return shapeTriageRows((data ?? []) as unknown as TriageQueueRow[]);
}

/**
 * Validate an untrusted decide body into a {@link DecideRequest}, or throw a
 * {@link TriageError} 400 on a bad status / missing required field. `approve`
 * requires a `distribution_scope`; `decline` requires a non-empty `reason`
 * (mirrors dojo-mind's decide validation). `revise` needs neither.
 */
export function parseDecideBody(body: Record<string, unknown>): DecideRequest {
	const status = body.status;
	if (status !== 'approve' && status !== 'revise' && status !== 'decline') {
		throw new TriageError(400, 'status must be approve, revise, or decline');
	}
	if (status === 'approve' && body.distribution_scope == null) {
		throw new TriageError(400, 'distribution_scope is required to approve');
	}
	if (status === 'decline') {
		const reason = typeof body.reason === 'string' ? body.reason.trim() : '';
		if (!reason) throw new TriageError(400, 'a reason is required to decline');
	}
	return {
		status,
		distribution_scope: body.distribution_scope,
		reason: typeof body.reason === 'string' ? body.reason : undefined
	};
}

/** The past-tense verdict + the terminal queue state each decision moves the row to. */
const DECIDE_MAP: Record<DecideStatus, { result: DecideResult['status']; state: string }> = {
	approve: { result: 'approved', state: 'resolved' },
	revise: { result: 'revised', state: 'in_review' },
	decline: { result: 'declined', state: 'resolved' }
};

/**
 * Record a maintainer decision for a triage signature: locate the OPEN queue row
 * for `(tenant, signature)`, write a `dojo.decisions` row, and flip the queue
 * `state` (approve/decline → resolved; revise → in_review). The port of
 * dojo-mind's decide. Returns the flattened `DecideResult`; on `approved` it
 * carries the artifact id (the promote/publish sweep advances `seq` separately,
 * so `seq` is omitted here rather than faked — a divergence noted like the rules
 * republish-seq follow-up).
 *
 * A 404 is thrown when no open row matches the signature (already decided, or an
 * unknown signature) — the caller maps it.
 */
export async function decideTriage(
	db: DojoClient,
	tenantId: string,
	signature: string,
	req: DecideRequest,
	maintainerId: string
): Promise<DecideResult> {
	// Find the open queue row for this signature (tenant-scoped).
	const { data: row, error: findErr } = await db
		.from('triage_queue')
		.select('id, artifact_id, state')
		.eq('tenant_id', tenantId)
		.eq('signature', signature)
		.in('state', OPEN_STATES as unknown as string[])
		.maybeSingle();
	if (findErr) throw new TriageError(500, findErr.message);
	if (!row) throw new TriageError(404, 'no open triage row for that signature');

	const queueRow = row as { id: string; artifact_id: string; state: string };
	const mapped = DECIDE_MAP[req.status];

	// Write the decision row (non-repudiation trail).
	const { error: decErr } = await db.from('decisions').insert({
		tenant_id: tenantId,
		artifact_id: queueRow.artifact_id,
		triage_id: queueRow.id,
		maintainer_id: maintainerId,
		status: req.status,
		reason: req.reason ?? null,
		distribution_scope: req.distribution_scope ?? null,
		automated: false
	});
	if (decErr) throw new TriageError(500, decErr.message);

	// Flip the queue state to reflect the verdict.
	const { error: updErr } = await db
		.from('triage_queue')
		.update({ state: mapped.state, updated_at: new Date().toISOString() })
		.eq('id', queueRow.id);
	if (updErr) throw new TriageError(500, updErr.message);

	const result: DecideResult = { status: mapped.result };
	if (req.status === 'approve') result.artifact_id = queueRow.artifact_id;
	return result;
}
