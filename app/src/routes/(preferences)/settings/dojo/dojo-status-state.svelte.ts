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
export type SyncTone = 'success' | 'warn' | 'danger' | 'info' | 'muted' | 'ink';

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
    case 'partial':
      return 'warn';
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

const SYNC_TONE_CLASS: Record<SyncTone, string> = {
  success: 'text-success',
  warn: 'text-warning',
  danger: 'text-danger',
  info: 'text-info',
  muted: 'text-ink-soft',
  ink: 'text-ink-soft',
};

/** The text colour a sync row's agreement line wears. */
export function syncToneClass(state: string): string {
  return SYNC_TONE_CLASS[syncTone(state)];
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
  // A partial push IS an agreement — on everything but what the detail names.
  if (row.state === 'partial') return agreed ? `partly agreed ${agreed}` : 'partly agreed';
  if (!agreed) return 'never agreed';
  return `agreed ${agreed}`;
}

/** What each recorded skip reason means for the checkout it names (#225). The
 *  daemon records a code; a code is not something a reader should have to look up. */
const SKIP_REASONS: Record<string, string> = {
  not_a_tracked_branch: 'this branch is not one the dōjō tracks, so its graph stays on this machine',
  index_not_current: 'the checkout moved to another branch since it was indexed; it syncs once re-indexed',
};

/**
 * Pure: the line under a row — a skipped row's reason in words, a failed row's
 * message as recorded. Only a SKIP is translated: a failure message is the
 * fault's own text, even if it happens to spell a code. A server-joined remedy
 * wins over the local reason map, and an unknown reason passes through
 * unchanged rather than being hidden. `null` when there is nothing to say.
 */
export function detailLine(row: SyncStateRow): string | null {
  if (row.last_error === null) return null;
  if (row.state === 'skipped') return row.remedy ?? SKIP_REASONS[row.last_error] ?? row.last_error;
  return row.last_error;
}
