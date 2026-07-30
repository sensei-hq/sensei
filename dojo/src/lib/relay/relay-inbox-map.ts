// Wire → RelaySession mapping (the real Load's transform). Pure + tested without a
// backend. The inbox API is single-tenant (relay-data.ts); the Load fans these out
// over the user's memberships and maps each run + its gates + its segments into the
// UI-shaped RelaySession the components/state already consume.
import type { RelayRun, RelayGate, RelaySegment, RelayRunStatus } from '$lib/relay-data';
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

/** RelayGate → RelayAsk. Prompt/context/options come from the stripped payload; the
 *  held-task title is resolved from the run's segments by segment_id. */
export function toAsk(gate: RelayGate, segTitleById: Map<string, string>): RelayAsk {
	const p = gate.payload ?? {};
	const options = Array.isArray(p.options) ? p.options.map((o) => String(o)) : [];
	return {
		id: gate.id,
		action: ASK_ACTION[gate.kind] ?? 'choose',
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
		asks: gates.map((g) => toAsk(g, segTitleById))
	};
}
