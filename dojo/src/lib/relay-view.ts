// Pure presentation helpers for the maintainer/console Relay run-list screen (P2).
//
// The phone speaks plain language, so the internal run status (dojo.relay_run_status)
// is mapped to a friendly UI label + a token-utility tone CLASS (text-accent /
// text-success / …) per the console design system — never a raw hex/oklch. All
// functions are side-effect-free (data in → display value out) so they unit test
// without a DOM or a live backend and the screen stays declarative.

import type { RelayRunStatus, SegmentState, RelayGate } from './relay-data';

// ── run status → friendly label + tone class ─────────────────────────────────

interface StatusBadge {
	/** The plain-language label shown on the pill. */
	label: string;
	/** Token utility class tinting the pill (text-*, matching -soft fill via DojoChip). */
	toneClass: string;
}

// The mockup (relay.jsx / dojo-relay.jsx) speaks in plain language: running reads
// "Running", stalled reads "Stuck", blocked reads "Needs you". crashed ≠ failed —
// both read danger but stay distinct labels.
const STATUS_BADGES: Record<RelayRunStatus, StatusBadge> = {
	running: { label: 'Running', toneClass: 'text-accent' },
	paused: { label: 'Paused', toneClass: 'text-ink-mute' },
	stalled: { label: 'Stuck', toneClass: 'text-warning' },
	crashed: { label: 'Crashed', toneClass: 'text-danger' },
	blocked: { label: 'Needs you', toneClass: 'text-accent' },
	done: { label: 'Done', toneClass: 'text-success' },
	failed: { label: 'Failed', toneClass: 'text-danger' }
};

const FALLBACK_BADGE: StatusBadge = { label: 'Unknown', toneClass: 'text-ink-mute' };

/** Friendly label + tone class for a run status (defaults to a muted "Unknown"). */
export function statusBadge(status: RelayRunStatus): StatusBadge {
	return STATUS_BADGES[status] ?? FALLBACK_BADGE;
}

// ── segment state → friendly label + tone class ─────────────────────────────

// The outline (relay.jsx / relay-planner.jsx) reads in the same plain vocabulary
// as the run status: a done step is "Done", the running one is "In progress", a
// step still ahead is "Queued". needs_review + blocked both rise as "Needs you"
// in the accent tone (they gate on the maintainer); failed reads danger; skipped
// stays muted (deliberately passed over). Same shape as statusBadge so the markup
// stays declarative and the mapping is unit-tested in one place.
const SEGMENT_STATE_BADGES: Record<SegmentState, StatusBadge> = {
	pending: { label: 'Queued', toneClass: 'text-ink-mute' },
	active: { label: 'In progress', toneClass: 'text-success' },
	done: { label: 'Done', toneClass: 'text-ink-mute' },
	skipped: { label: 'Skipped', toneClass: 'text-ink-faint' },
	failed: { label: 'Failed', toneClass: 'text-danger' },
	blocked: { label: 'Needs you', toneClass: 'text-accent' },
	needs_review: { label: 'Needs you', toneClass: 'text-accent' }
};

/** Friendly label + tone class for a segment state (defaults to a muted "Unknown"). */
export function segmentStateBadge(state: SegmentState): StatusBadge {
	return SEGMENT_STATE_BADGES[state] ?? FALLBACK_BADGE;
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
 * grows one. Mirrors RelayGateCard's runHref.
 */
export function gateHref(gate: RelayGate): string | null {
	return gate.run_id && gate.run_id !== ZERO_UUID ? `/you/runs/${gate.run_id}` : null;
}
