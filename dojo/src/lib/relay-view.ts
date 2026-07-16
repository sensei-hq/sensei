// Pure presentation helpers for the maintainer/console Relay run-list screen (P2).
//
// The phone speaks plain language, so the internal run status (dojo.relay_run_status)
// is mapped to a friendly UI label + a token-utility tone CLASS (text-accent /
// text-success / …) per the console design system — never a raw hex/oklch. All
// functions are side-effect-free (data in → display value out) so they unit test
// without a DOM or a live backend and the screen stays declarative.

import type { RelayRunStatus, SegmentState } from './relay-data';

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
