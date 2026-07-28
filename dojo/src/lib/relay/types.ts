// Inbox domain types — what the UI renders, NOT the wire shape. The Load layer
// (relay-inbox.ts) maps `relay_sessions`/`relay_inbox`/`relay_segments` → these.
// Keeping them UI-shaped is what lets components + tests ignore the API and lets
// the mock/real Load swap without touching component or state.

export type RelayStatus = 'running' | 'stalled' | 'blocked' | 'paused' | 'done' | 'failed';

/** Mirrors dojo.segment_state — a node's roll-up state in the plan graph. */
export type SegmentState =
	| 'pending'
	| 'active'
	| 'done'
	| 'skipped'
	| 'failed'
	| 'blocked'
	| 'needs_review';

export interface RelayTask {
	id: string;
	title: string;
	state: SegmentState;
	agent?: string;
	model?: string;
	specRef?: string;
	summary?: string;
}

export interface RelayPhase {
	id: string;
	title: string;
	state: SegmentState;
	tasks: RelayTask[];
}

/** One run's plan (phases → tasks) — drives the pips (rail) + the outline (detail). */
export interface SegmentGraph {
	phases: RelayPhase[];
}

// Ask kinds are ACTIONS (verbs), distinct from run STATUS (state nouns). Wire kinds
// map to these: approval→approve · decision→choose · stall→resume · chat/nudge→chat.
// (Jerry: status ≠ action; a "stalled" state takes the "resume" action.)
export type AskAction = 'approve' | 'choose' | 'resume' | 'chat';

export interface RelayAsk {
	id: string;
	action: AskAction;
	blocking: boolean;
	prompt: string;
	context?: string;
	/** Selectable options (rokkit schema form) — picking one sends the reply. */
	options: string[];
	/** The task this ask holds, for the "holds {task} →" link + plan cross-jump. */
	segmentId?: string;
	taskTitle?: string;
	createdAt: string;
}

export interface RelaySession {
	/** The daemon run id (relay_sessions.run_id). */
	id: string;
	/** Repo/project name — user+tenant scoped; RLS keeps it to the viewer's own. */
	project: string | null;
	title: string;
	goal: string | null;
	status: RelayStatus;
	done: number;
	total: number;
	phase: string | null;
	/** Relative "last activity" instant (ISO) — the age label + staleness. */
	lastEventAt: string | null;
	/** Count of open asks (drives "N need you" + the needs filter). */
	needs: number;
	/** A non-gate attention state, or null when nothing is wrong. */
	attention: 'stalled' | 'blocked' | 'failed' | null;
	plan: SegmentGraph;
	asks: RelayAsk[];
}

export type RelayFilter = 'needs' | 'running' | 'finished' | 'all';
