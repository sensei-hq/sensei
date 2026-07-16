// Relay console API client + types (P2) — the phone/console (Supabase-JWT) plane.
// In-Worker /v1 routes (src/routes/v1/t/[origin]/[org]/relay/…):
//   GET  {DOJO}/v1/t/{tenant}/relay/session[?run_id]  → { sessions: RelayRun[] }
//   GET  {DOJO}/v1/t/{tenant}/relay/segments?run_id    → { segments: RelaySegment[] }
//   POST {DOJO}/v1/t/{tenant}/relay/review             → { submitted: number }
//   POST {DOJO}/v1/t/{tenant}/relay/reply              → { id, seq }
// Types mirror dojo_protocol::relay + the dojo.relay_* DDL — not invented. Shared
// HTTP primitives (base URL, bearer, tenant encoding, error, JSON parse) live in
// dojo-api.ts and are reused. Plain module (fetch injectable) so it unit-tests
// without a live backend and imports cleanly under SSR.
import { DojoApiError, authHeaders, dojoApiUrl, encodeTenant, parseJson, type DojoCallOpts } from './dojo-api';

export { DojoApiError };

/** Run status — mirrors dojo.relay_run_status (crashed ≠ failed). */
export type RelayRunStatus = 'running' | 'paused' | 'stalled' | 'crashed' | 'blocked' | 'done' | 'failed';

/** A supervised run as the phone sees it — dojo.relay_sessions. */
export interface RelayRun {
	id: string;
	run_id: string;
	title: string;
	goal: string | null;
	status: RelayRunStatus;
	progress_done: number;
	progress_total: number;
	current_phase: string | null;
	current_feature: string | null;
	last_event_at: string | null;
	paused_until: string | null;
	pause_reason: string | null;
	started_at: string;
	completed_at: string | null;
}

/** Segment state — mirrors dojo.segment_state. */
export type SegmentState = 'pending' | 'active' | 'done' | 'skipped' | 'failed' | 'blocked' | 'needs_review';

/** One outline node (Phase or, nested, Step) — dojo.relay_segments. */
export interface RelaySegment {
	id: string;
	parent_id: string | null;
	seq: number;
	title: string;
	summary: string | null;
	detail: string | null;
	state: SegmentState;
	is_gate: boolean;
	gate_severity: 'blocking' | 'advisory' | null;
	response_verdict: string | null;
	response_note: string | null;
	submitted_at: string | null;
}

/** One per-segment review verdict in the PR-review "Send" batch. */
export interface SegmentReview {
	seq: number;
	verdict: 'approve' | 'request_changes' | 'comment';
	note?: string;
}

function base(tenantKey: string): string {
	return `${dojoApiUrl}/v1/t/${encodeTenant(tenantKey)}/relay`;
}

/** Every run the caller can see in this tenant (newest first). */
export async function listRuns(tenantKey: string, opts: DojoCallOpts = {}): Promise<RelayRun[]> {
	const f = opts.fetch ?? fetch;
	const res = await f(`${base(tenantKey)}/session`, { headers: authHeaders(opts.accessToken) });
	const { sessions } = await parseJson<{ sessions: RelayRun[] }>(res);
	return sessions ?? [];
}

/** A run's outline, ordered by seq. */
export async function getSegments(tenantKey: string, runId: string, opts: DojoCallOpts = {}): Promise<RelaySegment[]> {
	const f = opts.fetch ?? fetch;
	const res = await f(`${base(tenantKey)}/segments?run_id=${encodeURIComponent(runId)}`, {
		headers: authHeaders(opts.accessToken)
	});
	const { segments } = await parseJson<{ segments: RelaySegment[] }>(res);
	return segments ?? [];
}

/** Submit the PR-review batch (per-segment verdicts). Returns the count applied. */
export async function submitReview(
	tenantKey: string,
	runId: string,
	reviews: SegmentReview[],
	opts: DojoCallOpts = {}
): Promise<number> {
	const f = opts.fetch ?? fetch;
	const res = await f(`${base(tenantKey)}/review`, {
		method: 'POST',
		headers: { 'content-type': 'application/json', ...authHeaders(opts.accessToken) },
		body: JSON.stringify({ run_id: runId, reviews })
	});
	const { submitted } = await parseJson<{ submitted: number }>(res);
	return submitted;
}

/** Answer a gate (approve/deny/decision/chat). The daemon consumes the reply. */
export async function replyToGate(
	tenantKey: string,
	inboxId: string,
	reply: unknown,
	opts: DojoCallOpts = {}
): Promise<void> {
	const f = opts.fetch ?? fetch;
	const res = await f(`${base(tenantKey)}/reply`, {
		method: 'POST',
		headers: { 'content-type': 'application/json', ...authHeaders(opts.accessToken) },
		body: JSON.stringify({ inbox_id: inboxId, reply })
	});
	await parseJson<{ id: string }>(res);
}
