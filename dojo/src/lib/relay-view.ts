// Pure presentation helpers for the maintainer/console Relay run-list screen (P2).
//
// The phone speaks plain language, so the internal run status (dojo.relay_run_status)
// is mapped to a friendly UI label + a token-utility tone CLASS (text-accent /
// text-success / …) per the console design system — never a raw hex/oklch. All
// functions are side-effect-free (data in → display value out) so they unit test
// without a DOM or a live backend and the screen stays declarative.

import type { RelayRunStatus, SegmentState, RelayGate } from './relay-data';
import type { RelaySession } from './relay/types';
import { K2_NODE, K2_STATUS, taskState } from '$lib/components/kit/vocab';
import { relativeAge } from '$lib/triage/view';
import * as m from '$lib/paraglide/messages';

// ── run status → friendly label + tone class ─────────────────────────────────

interface StatusBadge {
	/** The plain-language label shown on the pill. */
	label: string;
	/** Token utility class tinting the pill (text-*, matching -soft fill via DojoChip). */
	toneClass: string;
	/** A live run's pulsing dot, in the run's own tone; absent otherwise. */
	pulseClass?: string;
}

// The mockup (relay.jsx / dojo-relay.jsx) speaks in plain language: running reads
// "Running", stalled reads "Stuck", blocked reads "Needs you". crashed ≠ failed —
// both read danger but stay distinct labels.
// running is success (the mockup's jade, the list's dot): accent means "needs
// you", which a running run does not (audit 2026-10-28, item 5). blocked IS
// "needs you" — dojo.relay_run_status: "waiting on a hard-block gate".
const STATUS_BADGES: Record<RelayRunStatus, StatusBadge> = {
	running: { label: 'Running', toneClass: 'text-success', pulseClass: 'bg-success' },
	paused: { label: 'Paused', toneClass: 'text-ink-soft' },
	stalled: { label: 'Stuck', toneClass: 'text-warning' },
	crashed: { label: 'Crashed', toneClass: 'text-danger' },
	blocked: { label: 'Needs you', toneClass: 'text-accent' },
	done: { label: 'Done', toneClass: 'text-ink-soft' },
	failed: { label: 'Failed', toneClass: 'text-danger' }
};

const FALLBACK_BADGE: StatusBadge = { label: 'Unknown', toneClass: 'text-ink-soft' };

/** Friendly label + tone class for a run status (defaults to a muted "Unknown"). */
export function statusBadge(status: RelayRunStatus): StatusBadge {
	return STATUS_BADGES[status] ?? FALLBACK_BADGE;
}

// ── segment state → label, text tone and pip ──────────────────────────────────

// Every step colour is the kit's K2_NODE — the table the detail's outline renders —
// so the list's pips, the detail's badges and the outline agree (audit 2026-10-28,
// item 5: active was accent in the pips and success in the badge; blocked danger in
// one, accent in another). The labels are the phone's plain language; only the step
// that gates on you reads "Needs you".
export interface SegmentView extends StatusBadge {
	/** The step's pip fill; a hollow hairline pip when it has not run. */
	pipClass: string;
}

const SEGMENT_LABELS: Record<SegmentState, string> = {
	pending: 'Queued',
	active: 'In progress',
	done: 'Done',
	skipped: 'Skipped',
	failed: 'Failed',
	blocked: 'Blocked',
	needs_review: 'Needs you'
};

const HOLLOW_PIP = 'border border-paper-edge bg-transparent';

/** Label, text tone and pip for a step state (a muted "Unknown" for anything else). */
export function segmentStateBadge(state: SegmentState): SegmentView {
	const label = SEGMENT_LABELS[state];
	if (!label) return { ...FALLBACK_BADGE, pipClass: HOLLOW_PIP };
	const tone = K2_NODE[taskState(state)];
	const hollow = state === 'pending' || state === 'skipped';
	return { label, toneClass: tone.text, pipClass: hollow ? HOLLOW_PIP : tone.fill };
}

// ── the inbox list card ──────────────────────────────────────────────────────

export interface SessionView {
	dotClass: string;
	why: string;
	whyClass: string;
	titleClass: string;
	age: string;
	pips: { id: string; pipClass: string }[];
	count: string;
}

/**
 * One list card, decided here. A gate waiting on you is accent; otherwise a
 * non-gate problem takes the kit's run-status tone (stalled and blocked warn,
 * failed is danger); a run going well is success, as the detail's badge.
 */
export function sessionView(session: RelaySession): SessionView {
	const problem = session.attention ? K2_STATUS[session.attention] : null;
	const statusLabels: Record<RelaySession['status'], string> = {
		running: m.status_running(),
		stalled: m.status_stalled(),
		blocked: m.status_blocked(),
		paused: m.status_paused(),
		done: m.status_done(),
		failed: m.status_failed()
	};
	const why =
		session.needs > 0
			? session.needs === 1
				? m.card_needs_one()
				: m.card_needs_many({ count: session.needs })
			: session.attention === 'stalled'
				? m.card_no_heartbeat()
				: session.attention === 'blocked'
					? m.card_blocked()
					: session.attention === 'failed'
						? m.card_failed()
						: statusLabels[session.status];
	return {
		dotClass:
			session.needs > 0
				? 'bg-accent'
				: problem
					? problem.fill
					: session.status === 'running'
						? K2_STATUS.running.fill
						: 'border border-ink-faint',
		why,
		whyClass:
			session.needs > 0
				? 'text-accent font-semibold'
				: problem
					? `${problem.text} font-semibold`
					: 'text-ink-soft',
		titleClass: session.status === 'done' ? 'text-ink-soft' : 'text-ink font-medium',
		age: session.lastEventAt ? relativeAge(session.lastEventAt) : '',
		pips: session.plan.phases.map((p) => ({ id: p.id, pipClass: segmentStateBadge(p.state).pipClass })),
		count: `${session.done}/${session.total}`
	};
}

// ── progress (done / total → clamped CSS percentage) ─────────────────────────

/**
 * Progress bar width as a CSS percentage string, clamped to 0–100. A non-positive
 * or missing total reads as 0% (nothing to show) rather than dividing by zero.
 */
export function progressWidth(done: number, total: number): string {
	if (!Number.isFinite(total) || total <= 0) return '0%';
	const ratio = Math.max(0, Math.min(1, done / total));
	return `${Math.round(ratio * 100)}%`;
}

/** A run's progress, from its own authoritative counts: the stage is the first
 *  phase not done (its plan is summarised to one entry per phase). */
export function runProgress(run: Pick<RelaySession, 'done' | 'total' | 'plan'>): {
	stage: number;
	stages: number;
	pct: number;
	width: string;
} {
	const stages = run.plan.phases.length;
	const active = run.plan.phases.findIndex((p) => p.state !== 'done');
	const width = progressWidth(run.done, run.total);
	return { stage: active === -1 ? stages : active + 1, stages, pct: parseInt(width, 10), width };
}

// ── "blocked on you" home: gate urgency (P4.6) ───────────────────────────────
//
// The away-from-keyboard landing aggregates the pending gates (relay-data.listGates
// → GET /v1/relay/gates) across every run and surfaces what's waiting on the human,
// most-urgent first. All logic here is pure (gates in → ordered gates / counts /
// href out) so RelayBlockedHome stays declarative and the ordering is unit-tested
// in one place — never buried in the component.

const ZERO_UUID = '00000000-0000-0000-0000-000000000000';

/** A gate's urgency band — blocking halts the run (needs you), advisory is a heads-up. */
export type GateSeverity = 'blocking' | 'advisory';

// The needs-you kinds that halt the run — same set the P4.4 push trigger notifies on
// (relay/inbox PUSH_KINDS). chat/nudge are conversational, so they read as advisory.
const BLOCKING_KINDS = new Set(['approval', 'decision', 'stall']);

/**
 * A gate's severity for ordering. Prefers an explicit `payload.gate_severity`
 * (mirrors dojo.gate_severity on the outline segment); when absent or malformed it
 * falls back to the kind — approval/decision/stall block the run, everything else is
 * advisory (a garbage severity is treated as absent, never trusted).
 */
export function gateSeverity(gate: RelayGate): GateSeverity {
	const raw = gate.payload?.gate_severity;
	if (raw === 'blocking' || raw === 'advisory') return raw;
	return BLOCKING_KINDS.has(gate.kind) ? 'blocking' : 'advisory';
}

/**
 * Gates ordered by urgency for the "blocked on you" home: blocking before advisory,
 * then oldest-waiting first within each band (a gate left waiting longest is the most
 * urgent). Pure, deterministic, and stable — equal keys keep their input order, and
 * the input array is not mutated.
 */
export function orderGatesByUrgency(gates: RelayGate[]): RelayGate[] {
	const rank = (g: RelayGate) => (gateSeverity(g) === 'blocking' ? 0 : 1);
	return [...gates].sort((a, b) => {
		const bandDelta = rank(a) - rank(b);
		if (bandDelta !== 0) return bandDelta;
		// Oldest first: an earlier created_at sorts ahead. localeCompare on ISO-8601
		// is a correct chronological order and keeps ties stable (returns 0).
		return a.created_at.localeCompare(b.created_at);
	});
}

/** Counts for the home header ("3 waiting on you"): total split into blocking/advisory. */
export function blockedSummary(gates: RelayGate[]): {
	total: number;
	blocking: number;
	advisory: number;
} {
	let blocking = 0;
	for (const g of gates) if (gateSeverity(g) === 'blocking') blocking++;
	return { total: gates.length, blocking, advisory: gates.length - blocking };
}

/**
 * Deep-link for a gate's row — the run detail route, or null when the run id is
 * missing / the all-zeros uuid (no orphan link). The detail page has no per-gate
 * anchor yet, so this points at the run; add a `#…` fragment here if `[run_id]`
 * grows one.
 */
export function gateHref(gate: RelayGate): string | null {
	return gate.run_id && gate.run_id !== ZERO_UUID ? `/you/runs/${gate.run_id}` : null;
}
