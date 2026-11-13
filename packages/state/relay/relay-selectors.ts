// The relay selectors (#421; design 2026-11-12 §2b): pure functions from the read to
// what a screen shows, shared by the desktop and the dōjō. Numbers are counted from
// the read the daemon gave, never guessed; a number it did not give reads as such (D83).
//
// The row selectors for each view (lanes, waits, queue rows, the run card, members,
// the task view) are built with their views (#422–#425).

import type { RelayLane, RelayNow } from './types.js';

/** The four stats on Now (CONTRACT §1c-0), before the shell draws them. */
export interface RelayStats {
  /** Lanes on a task, or holding a review. */
  working: number;
  /** Reports waiting for a reviewer and idle lanes; how long the oldest report has waited. */
  waiting: { count: number; oldestMin: number | null };
  /** Open decisions and lanes whose lease is lost. */
  blocked: number;
  /** Tasks completed today. */
  done: number;
  firstPass: string;
}

/** Whole minutes from `iso` to `at`, never negative. */
export const minutesSince = (iso: string, at: number): number =>
  Math.max(0, Math.round((at - new Date(iso).getTime()) / 60_000));

/** On a task, or holding a review: working. */
export const isWorking = (lane: RelayLane): boolean =>
  lane.lease_state !== 'expired' &&
  (Boolean(lane.reviewing) || (lane.task != null && ['working', 'assigned', 'rework'].includes(lane.task.state)));

/** Reachable, nothing claimed, nothing to review: idle (lanes.md §15 `no_work`). */
export const isIdle = (lane: RelayLane): boolean =>
  lane.lease_state !== 'expired' && !lane.limit_scope && !lane.task && !lane.reviewing;

/** "1 of 5 <suffix>": first-pass approvals of the reviews today; no review reported reads so. */
export function firstPassLine(now: RelayNow, suffix: string): string {
  const { reviewed, first_pass_approved } = now.today;
  // D83: a number nobody reported reads so, never "0 of".
  if (reviewed == null) return 'first pass not reported';
  return `${first_pass_approved ?? 0} of ${reviewed} ${suffix}`;
}

/** The four stats counted from the read at `at`; null without a read. */
export function toStats(now: RelayNow | null, at: number): RelayStats | null {
  if (!now) return null;
  return {
    working: now.lanes.filter(isWorking).length,
    waiting: {
      count: now.floating_reviews.length + now.lanes.filter(isIdle).length,
      oldestMin: now.floating_reviews.length
        ? Math.max(...now.floating_reviews.map((r) => minutesSince(r.reported_at, at)))
        : null,
    },
    blocked: now.decisions.length + now.lanes.filter((l) => l.lease_state === 'expired').length,
    done: now.completed_today.length,
    firstPass: firstPassLine(now, 'first-pass'),
  };
}
