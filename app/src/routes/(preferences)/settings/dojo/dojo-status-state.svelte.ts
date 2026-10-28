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
 * fault's own text, even if it happens to spell a code. An unknown reason passes
 * through unchanged rather than being hidden. `null` when there is nothing to say.
 */
export function detailLine(row: SyncStateRow): string | null {
  if (row.last_error === null) return null;
  if (row.state === 'skipped') return SKIP_REASONS[row.last_error] ?? row.last_error;
  return row.last_error;
}

/** Rank of a sync state for "what most needs saying". Lower sorts first.
 *  `pending` outranks `skipped`: one is unfinished, the other is settled. */
const STATE_RANK: Record<string, number> = {
  error: 0,
  pending: 1,
  partial: 2,
  skipped: 3,
  synced: 4,
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

  const partial = counts.partial ?? 0;
  if (partial > 0) return `${total} tracked · ${partial} partial`;

  const skipped = counts.skipped ?? 0;
  if (skipped > 0) return `${total} tracked · ${skipped} skipped`;

  return `${total} tracked · all agreed`;
}

/** An identity's own sync entities — about the identity, not a repository. */
const IDENTITY_ENTITIES = new Set(['forge_mirror', 'dojo_sync_plan']);

export type SyncItem = { row: SyncStateRow; branch: string | null };
export type SyncDay = {
  /** `YYYY-MM-DD` it last agreed, else last tried; `null` for neither. */
  day: string | null;
  /** The identity's own entities that day (its sync plan, forge mirror). */
  own: SyncItem[];
  repositories: { name: string; items: SyncItem[] }[];
};
export type SyncIdentity = {
  /** The identity's NAME; `null` for rows no sync has attributed yet. */
  label: string | null;
  slot: string | null;
  days: SyncDay[];
};

/**
 * Pure: the sync rows as identity → day → repository (user, 2026-10-27).
 *
 * An identity is named by its label, never its Keychain slot (`default` is a
 * slot, and read as "the default identity"). A repository's branches fold into
 * it: `repo` and `repo@develop` are one repository, the branch on its line.
 * Within a day rows keep the order they came in — worst first. Identities by
 * name; rows not yet attributed last.
 */
export function groupSync(rows: SyncStateRow[], identities: { slot: string | null; label: string }[]): SyncIdentity[] {
  const labelOf = (slot: string | null) =>
    slot === null ? null : identities.find((i) => i.slot === slot)?.label ?? slot;
  const groups = new Map<string, SyncIdentity>();
  for (const row of rows) {
    const label = labelOf(row.persona);
    const key = label ?? '\u0000';
    const identity = groups.get(key) ?? { label, slot: row.persona, days: [] };
    groups.set(key, identity);
    const day = isoDay(row.synced_at) ?? isoDay(row.attempted_at) ?? null;
    let d = identity.days.find((x) => x.day === day);
    if (!d) identity.days.push((d = { day, own: [], repositories: [] }));
    if (IDENTITY_ENTITIES.has(row.entity)) {
      d.own.push({ row, branch: null });
      continue;
    }
    const at = row.entity_key.lastIndexOf('@');
    const [name, branch] = at > 0 ? [row.entity_key.slice(0, at), row.entity_key.slice(at + 1)] : [row.entity_key, null];
    let r = d.repositories.find((x) => x.name === name);
    if (!r) d.repositories.push((r = { name, items: [] }));
    r.items.push({ row, branch });
  }
  const out = [...groups.values()];
  for (const g of out) {
    g.days.sort((a, b) => (a.day === b.day ? 0 : a.day === null ? 1 : b.day === null ? -1 : a.day < b.day ? 1 : -1));
    for (const d of g.days) d.repositories.sort((a, b) => a.name.localeCompare(b.name));
  }
  return out.sort((a, b) => (a.label === null ? 1 : b.label === null ? -1 : a.label.localeCompare(b.label)));
}
