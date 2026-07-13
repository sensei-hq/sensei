// Observatory · Share review — the upstream publish-gate presentation logic.
//
// Co-locates everything the `+page.svelte` / `ShareReviewScreen.svelte` template
// needs so those stay pure: destination + cadence copy, the queued-vs-held
// partition (held items never ship — they are shown, never presented as
// publishable), the dereference note, the publish-button label + large-batch
// confirmation threshold, the per-item publish-outcome chip, and the Publish
// action controller. Mirrors the sibling Upgrades state module
// (`../upgrades/dojo-upgrades-state.svelte.ts`); the shared type-pill +
// attribution helpers live in `$lib/dojo-artifacts`.
//
// Wire-API-wins: `state`, attribution.mode and the publish `result` arrive as
// free strings, so an unrecognised value degrades to a neutral chip rather than
// being dropped.

import type { ApiResult, SenseiApi } from '$lib/api.js';
import type { ChipClasses } from '$lib/dojo-artifacts.js';
import type { PublishBatchOutcome, ShareReviewItem } from '$lib/types.js';

/** Batches larger than this require an explicit confirmation before publishing
 *  (spec done-gate: "batches over N items require a confirmation dialog"). */
export const PUBLISH_CONFIRM_THRESHOLD = 10;

/** Pure: an item ships on publish only when it is `queued`. `held` (residual
 *  identifier risk) and any other state never ship — they are shown for
 *  transparency, never as publishable. */
export function isShippable(item: ShareReviewItem): boolean {
  return item.state === 'queued';
}

/** Pure: split a batch's items into the ones that will ship (`queued`) and the
 *  ones held back (everything else). Order within each group is preserved. */
export function partitionItems(items: ShareReviewItem[]): {
  shippable: ShareReviewItem[];
  held: ShareReviewItem[];
} {
  const shippable: ShareReviewItem[] = [];
  const held: ShareReviewItem[] = [];
  for (const it of items) (isShippable(it) ? shippable : held).push(it);
  return { shippable, held };
}

/** Pure: how many items in the batch will actually ship. */
export function shippableCount(items: ShareReviewItem[]): number {
  return items.reduce((n, it) => (isShippable(it) ? n + 1 : n), 0);
}

/** Pure: how many items are held back and won't ship this batch. */
export function heldCount(items: ShareReviewItem[]): number {
  return items.length - shippableCount(items);
}

/** Pure: named-token chip classes + label for an item's ship state. `queued`
 *  reads info (about to ship), `held` reads warning (won't ship); an unexpected
 *  state degrades to muted ink, still labelled with the raw wire value. */
export function shareStateChip(state: string): ChipClasses {
  switch (state) {
    case 'queued': return { bg: 'bg-info-soft',    text: 'text-info',     label: 'queued' };
    case 'held':   return { bg: 'bg-warning-soft', text: 'text-warning',  label: 'held' };
    default:       return { bg: 'bg-paper-mute',   text: 'text-ink-mute', label: state || 'unknown' };
  }
}

/** Pure: the dereference note for an item, or null. Client-work whose source
 *  reference is stripped shows "source dropped" — a display-only fact the user
 *  cannot override (the strip is mandatory and enforced by the daemon). */
export function dereferenceLabel(item: ShareReviewItem): string | null {
  return item.will_dereference ? 'source dropped' : null;
}

/** Pure: the batch's routed destinations as printable chips (the tenant keys),
 *  or an empty array when the batch is unbound (no memberships). */
export function destinationChips(destination: string[] | null | undefined): string[] {
  return (destination ?? []).map((d) => d.trim()).filter((d) => d.length > 0);
}

/** Pure: true when the batch has no routed destination — nothing can leave. */
export function isUnbound(destination: string[] | null | undefined): boolean {
  return destinationChips(destination).length === 0;
}

/** Pure: the next-batch timestamp as a short label, or '' when absent /
 *  unparseable. Takes the date portion of the RFC-3339 string directly (no
 *  `Date` construction) so the display-only label never shifts by timezone. */
export function nextBatchLabel(next_batch_at: string | null | undefined): string {
  if (!next_batch_at) return '';
  const m = /^(\d{4}-\d{2}-\d{2})/.exec(next_batch_at.trim());
  return m ? m[1] : '';
}

/** Pure: the Publish button label for a shippable count. Zero shippable items
 *  reads "Nothing to publish" (the button is disabled). */
export function publishButtonLabel(shippable: number): string {
  if (shippable <= 0) return 'Nothing to publish';
  return `Publish ${shippable} to Dōjō`;
}

/** Pure: whether publishing this many items requires an explicit confirmation
 *  step first (guards against a large batch firing on a single click). */
export function requiresConfirm(shippable: number): boolean {
  return shippable > PUBLISH_CONFIRM_THRESHOLD;
}

/** Human-readable label for each publish `result` tag. */
const RESULT_LABELS: Record<string, string> = {
  published: 'published',
  staged: 'staged',
  held_residual_risk: 'held',
  queued_retry: 'queued for retry',
  already_sent: 'already sent',
  error: 'error',
  no_destination: 'no destination',
};

/** Pure: named-token chip classes + label for a per-item publish outcome.
 *  `published` reads success, `held`/`no_destination` read warning, `error`
 *  reads danger, `queued`/`staged` read info, `already_sent` reads muted ink;
 *  an unrecognised result echoes the raw tag in muted ink (never dropped). */
export function publishResultChip(result: string): ChipClasses {
  const label = RESULT_LABELS[result] ?? result ?? 'unknown';
  switch (result) {
    case 'published':          return { bg: 'bg-success-soft', text: 'text-success', label };
    case 'held_residual_risk':
    case 'no_destination':     return { bg: 'bg-warning-soft', text: 'text-warning', label };
    case 'error':              return { bg: 'bg-danger-soft',  text: 'text-danger',  label };
    case 'queued_retry':
    case 'staged':             return { bg: 'bg-info-soft',    text: 'text-info',    label };
    default:                   return { bg: 'bg-paper-mute',   text: 'text-ink-mute', label };
  }
}

/** Pure: a short roll-up line for a publish outcome — only the non-zero
 *  counters, e.g. "2 published · 1 held". "nothing published" when every
 *  counter is zero. */
export function outcomeSummary(outcome: PublishBatchOutcome): string {
  const parts: string[] = [];
  if (outcome.published > 0) parts.push(`${outcome.published} published`);
  if (outcome.held > 0) parts.push(`${outcome.held} held`);
  if (outcome.queued > 0) parts.push(`${outcome.queued} queued`);
  if (outcome.already_sent > 0) parts.push(`${outcome.already_sent} already sent`);
  if (outcome.errored > 0) parts.push(`${outcome.errored} errored`);
  return parts.length > 0 ? parts.join(' · ') : 'nothing published';
}

/**
 * The Publish controller for one batch. Tracks the in-flight state, the last
 * error and the last outcome so the template stays a pure template. The api
 * client and the reload callback are injected, so it is unit-tested with a
 * hand-rolled mock and never reaches for a global. `publish` resolves `true`
 * once the injected `reload` has re-loaded the (now-sent) batch, storing the
 * per-item outcome for the "where it went" view; `false` on a wire failure, in
 * which case `error` carries the daemon's reason (a missing / not-approved
 * batch is a 4xx) and the batch is left in place for a retry.
 */
export class PublishBatchAction {
  /** True while a publish is in flight. Drives button disabling. */
  busy = $state(false);
  /** The last publish error, or null. Cleared at the start of each publish. */
  error = $state<string | null>(null);
  /** The last successful publish outcome, or null — the "watch it travel" data. */
  outcome = $state<PublishBatchOutcome | null>(null);

  #api: SenseiApi;
  #reload: () => Promise<void> | void;

  constructor(api: SenseiApi, reload: () => Promise<void> | void) {
    this.#api = api;
    this.#reload = reload;
  }

  async publish(batchId: string): Promise<boolean> {
    if (this.busy) return false; // one publish at a time
    this.busy = true;
    this.error = null;
    const res: ApiResult<PublishBatchOutcome> = await this.#api.publishBatch(batchId);
    this.busy = false;
    if (res.ok) {
      this.outcome = res.data;
      await this.#reload();
      return true;
    }
    this.error = res.error.message;
    return false;
  }
}
