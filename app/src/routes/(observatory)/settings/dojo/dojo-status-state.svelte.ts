// Settings · Dōjō — how a sync row reads.
//
// The CREDENTIAL half of this screen has no module of its own on purpose:
// `PersonaList` already owns `describe` (the "30 minutes left" line), `tone` (how
// alarming that is) and `actionLabel` (Connect / Renew / Sign in again), and a
// second copy here would drift from the thresholds the sign-in overlay uses.
// This file covers only what is new — `sensei.sync_state`, which until now had
// three writers and no reader at all.
//
// ## `skipped` is not `error`
//
// The one property everything here protects. Both mean "not synced", but one is
// a decision (a private repository) and one is a fault. A screen that paints them
// the same either cries wolf about every private repo or stays silent about real
// failures — the exact reason the column is a four-value vocabulary rather than a
// boolean.
//
// ## A failed row keeps its last agreement
//
// `mark_sync_error` deliberately does not clear `synced_at`, so a failing entity
// still says when the two sides last agreed. The lines below spend that: they
// report the agreement AND the failure, because "broken since Tuesday" and "never
// worked" need different responses and a single timestamp cannot tell them apart.

import { isoDay } from '$lib/dates.js';
import type { SyncStateRow } from '$lib/types.js';

/** Semantic tone for a sync state. `ink` = a value we did not expect but still
 *  show — silently dropping a row is how a broken sync becomes invisible. */
export type SyncTone = 'success' | 'danger' | 'info' | 'muted' | 'ink';

/**
 * Pure: sync state → tone.
 *
 * `skipped` is `muted`, NOT `danger`: it is a decision, and colouring it as a
 * fault would make every private repository look broken.
 */
export function syncTone(state: string): SyncTone {
  switch (state) {
    case 'synced':
      return 'success';
    case 'error':
      return 'danger';
    case 'pending':
      return 'info';
    case 'skipped':
      return 'muted';
    default:
      return 'ink';
  }
}

/** Pure: `sensei.sync_entity` → English. An unknown entity passes through with
 *  its underscores relaxed rather than blanking — a row nobody can name is still
 *  a row worth seeing. */
export function entityLabel(entity: string): string {
  if (entity === 'dojo_sync_plan') return 'dōjō sync plan';
  return entity.replace(/_/g, ' ');
}

/**
 * Pure: one line saying when this entity last agreed with the dōjō, and whether
 * it is agreeing now.
 *
 * A failing row reports BOTH — the last agreement is preserved by the writer
 * precisely so it can be spent here. A skipped row says `skipped` and never
 * `failing`, because it did not fail.
 */
export function agreementLine(row: SyncStateRow): string {
  const agreed = isoDay(row.synced_at);
  if (row.state === 'skipped') {
    return agreed ? `skipped · last agreed ${agreed}` : 'skipped';
  }
  if (row.state === 'error') {
    return agreed ? `failing · last agreed ${agreed}` : 'failing · never agreed';
  }
  if (!agreed) return 'never agreed';
  return `agreed ${agreed}`;
}

/** Rank of a sync state for "what most needs saying". Lower sorts first.
 *  `pending` outranks `skipped`: one is unfinished, the other is settled. */
const STATE_RANK: Record<string, number> = {
  error: 0,
  pending: 1,
  skipped: 2,
  synced: 3,
};

/**
 * Pure: the state that most needs saying, from a by-state tally.
 *
 * An unknown state is SKIPPED rather than ranked — treating a missing rank as 0
 * would let an unrecognised value outrank a real failure and dominate the line.
 * `null` for an empty tally: never a fabricated all-clear.
 */
export function worstSyncState(counts: Record<string, number>): string | null {
  let worst: string | null = null;
  for (const state of Object.keys(counts)) {
    if (!(state in STATE_RANK)) continue;
    if (worst === null || STATE_RANK[state] < STATE_RANK[worst]) worst = state;
  }
  return worst;
}

/**
 * Pure: the one-line summary above the list.
 *
 * Leads with what is wrong, because that is what a reader came for. "all agreed"
 * is claimed ONLY when something is actually tracked — over zero entities it
 * would report a healthy sync on an install that has never synced anything.
 */
export function summarise(counts: Record<string, number>): string {
  const total = Object.values(counts).reduce((a, b) => a + b, 0);
  if (total === 0) return 'nothing tracked yet';

  const failing = counts.error ?? 0;
  if (failing > 0) return `${total} tracked · ${failing} failing`;

  const pending = counts.pending ?? 0;
  if (pending > 0) return `${total} tracked · ${pending} pending`;

  const skipped = counts.skipped ?? 0;
  if (skipped > 0) return `${total} tracked · ${skipped} skipped`;

  return `${total} tracked · all agreed`;
}
