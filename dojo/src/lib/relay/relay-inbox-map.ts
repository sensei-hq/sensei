// Wire → RelaySession mapping (the real Load's transform). Pure + tested without a
// backend. The inbox API is single-tenant (relay-data.ts); the Load fans these out
// over the user's memberships and maps each run + its gates + its segments into the
// UI-shaped RelaySession the components/state already consume.
import type { RelayRun, RelayGate, RelaySegment, RelayRunStatus } from '$lib/relay-data';
import { segmentsToActivity } from '$lib/relay-map';
import type {
	RelaySession,
	RelayAsk,
	AskAction,
	SegmentGraph,
	RelayPhase,
	RelayStatus,
	SegmentState
} from './types';

// relay_inbox_kind → the UI's ask ACTION (verb). status ≠ action: a stalled run's
// gate takes "resume". nudge (human→agent) surfaces as a chat continuation.
const ASK_ACTION: Record<string, AskAction> = {
	approval: 'approve',
	decision: 'choose',
	stall: 'resume',
	chat: 'chat',
	nudge: 'chat'
};

/** RelayRunStatus → the UI status (crashed folds into failed). */
export function mapRunStatus(s: RelayRunStatus): RelayStatus {
	return (s === 'crashed' ? 'failed' : s) as RelayStatus;
}

/** A non-gate attention state, or null. */
function attentionOf(s: RelayRunStatus): RelaySession['attention'] {
	if (s === 'stalled') return 'stalled';
	if (s === 'blocked') return 'blocked';
	if (s === 'failed' || s === 'crashed') return 'failed';
	return null;
}

function str(v: unknown): string | undefined {
	return typeof v === 'string' && v.length ? v : undefined;
}

/** The two answers an approval offers when the gate names none. */
export const APPROVAL_OPTIONS = ['Approve', 'Deny'] as const;

/** RelayGate → RelayAsk. Prompt/context/options come from the stripped payload; the
 *  held-task title is resolved from the run's segments by segment_id. */
export function toAsk(gate: RelayGate, segTitleById: Map<string, string>): RelayAsk {
	const p = gate.payload ?? {};
	const action = ASK_ACTION[gate.kind] ?? 'choose';
	const given = Array.isArray(p.options) ? p.options.map((o) => String(o)) : [];
	// The daemon reads a reply fail-open (only an explicit deny blocks), so an
	// approval is always an explicit choice — never a free line read as allow.
	const options = given.length || action !== 'approve' ? given : [...APPROVAL_OPTIONS];
	return {
		id: gate.id,
		action,
		blocking: p.severity !== 'advisory',
		prompt: str(p.prompt) ?? gate.run_title ?? 'sensei needs you',
		context: str(p.context) ?? str(p.evidence),
		options,
		segmentId: gate.segment_id ?? undefined,
		taskTitle: gate.segment_id ? segTitleById.get(gate.segment_id) : undefined,
		createdAt: gate.created_at
	};
}

/** Flat relay_segments (phases = top-level, steps nested by parent_id) → the plan
 *  graph the rail pips + the detail outline render. Ordered by seq. */
export function toSessionPlan(segments: RelaySegment[]): SegmentGraph {
	const ordered = [...segments].sort((a, b) => a.seq - b.seq);
	const children = new Map<string, RelaySegment[]>();
	for (const s of ordered) {
		if (s.parent_id) {
			const list = children.get(s.parent_id) ?? [];
			list.push(s);
			children.set(s.parent_id, list);
		}
	}
	const tops = ordered.filter((s) => !s.parent_id);
	const phases: RelayPhase[] = tops.map((t) => ({
		id: t.id,
		title: t.title,
		state: t.state as SegmentState,
		tasks: (children.get(t.id) ?? []).map((c) => ({
			id: c.id,
			title: c.title,
			state: c.state as SegmentState,
			agent: c.agent ?? undefined,
			model: c.model ?? undefined,
			specRef: c.spec_ref ?? undefined,
			summary: c.summary ?? undefined
		}))
	}));
	return { phases };
}

/** One RelayRun + its pending gates + its segments → a RelaySession. */
export function toRelaySession(
	run: RelayRun,
	gates: RelayGate[],
	segments: RelaySegment[],
	tenantKey: string,
	project: string | null = null
): RelaySession {
	const segTitleById = new Map(segments.map((s) => [s.id, s.title]));
	return {
		id: run.run_id,
		project: project ?? run.current_feature ?? null,
		title: run.title,
		goal: run.goal,
		status: mapRunStatus(run.status),
		done: run.progress_done,
		total: run.progress_total,
		phase: run.current_phase,
		model: segments.find((s) => s.model)?.model ?? undefined,
		startedAt: run.started_at,
		lastEventAt: run.last_event_at,
		needs: gates.length,
		attention: attentionOf(run.status),
		plan: toSessionPlan(segments),
		asks: gates.map((g) => toAsk(g, segTitleById)),
		tenantKey,
		activity: segmentsToActivity(segments)
	};
}

/**
 * An answer to an ask as the daemon reads it (`dojo/gate.rs::decision_from_reply`,
 * fail-open — only `verdict: 'deny'` blocks). An approval is a verdict; a decision
 * is the chosen option, or the typed line; a resume or chat is text. A note rides
 * along.
 */
export function askReply(ask: RelayAsk, answer: string, note?: string): Record<string, unknown> {
	const a = answer.trim();
	let body: Record<string, unknown>;
	if (ask.action === 'approve' && /^(approve|deny)$/i.test(a)) body = { verdict: a.toLowerCase() };
	else if (ask.options.includes(a)) body = { choice: a };
	else body = { text: a };
	const n = note?.trim();
	return n ? { ...body, note: n } : body;
}
