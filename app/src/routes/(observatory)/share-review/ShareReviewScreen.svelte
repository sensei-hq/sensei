<script lang="ts">
  import { PageHeader, EmptyState, ScreenState } from '$lib/components';
  import { Button } from '@rokkit/ui';
  import { typePill, attributionSummary, type ChipClasses } from '$lib/dojo-artifacts.js';
  import type { ShareReviewBatch, ShareReviewItem, PublishBatchOutcome } from '$lib/types.js';
  import {
    PublishBatchAction,
    partitionItems,
    shippableCount,
    heldCount,
    shareStateChip,
    dereferenceLabel,
    destinationChips,
    isUnbound,
    nextBatchLabel,
    publishButtonLabel,
    requiresConfirm,
    publishResultChip,
    outcomeSummary,
  } from './share-review-state.svelte.js';

  interface Props {
    /** The next approved-but-unsent batch, or null when nothing is pending. */
    batch: ShareReviewBatch | null;
    /** The Publish controller — api + reload injected by the page. */
    actions: PublishBatchAction;
    /** A LOAD failure (F8) — distinct from `batch: null` (honest-empty). When
     *  set, the screen shows error-with-Retry instead of the empty state. */
    loadError?: string | null;
    /** Retry callback for the load-error state (the page injects invalidateAll). */
    onretry?: () => void;
  }
  let { batch, actions, loadError = null, onretry }: Props = $props();

  // A large batch takes a second click: the first flips into a confirm prompt.
  let confirming = $state(false);

  const grouped = $derived(
    batch ? partitionItems(batch.items) : { shippable: [], held: [] },
  );
  const nShip = $derived(batch ? shippableCount(batch.items) : 0);
  const nHeld = $derived(batch ? heldCount(batch.items) : 0);
  const dests = $derived(destinationChips(batch?.destination));
  const unbound = $derived(isUnbound(batch?.destination));
  const when = $derived(nextBatchLabel(batch?.next_batch_at));
  const needsConfirm = $derived(requiresConfirm(nShip));
  // Publish is live only with something to ship, a routed destination, and no
  // in-flight publish.
  const canPublish = $derived(nShip > 0 && !unbound && !actions.busy);

  async function onPublish(): Promise<void> {
    if (!batch) return;
    // Large batch: first click asks for confirmation, doesn't fire.
    if (needsConfirm && !confirming) {
      confirming = true;
      return;
    }
    confirming = false;
    await actions.publish(batch.batch_id);
  }
</script>

{#snippet chip(cls: ChipClasses)}
  <span class="font-mono text-xs px-2 py-0.5 rounded-full {cls.bg} {cls.text}">{cls.label}</span>
{/snippet}

<!-- One item about to leave (or held). Held items are dimmed, carry a "won't
     ship" note, and never expose a publish affordance — the daemon's hold is
     shown, never overridable here. -->
{#snippet itemRow(item: ShareReviewItem, held: boolean)}
  {@const pill = typePill(item.type)}
  {@const deref = dereferenceLabel(item)}
  <article
    data-share-item={item.memory_id}
    data-state={item.state}
    class="border rounded-lg bg-paper-soft p-4 mb-3 {held
      ? 'opacity-70 border-warning-soft'
      : 'border-paper-edge'}"
  >
    <div class="flex items-center gap-2 mb-2">
      <span class="inline-flex items-center gap-1 font-mono text-xs px-2 py-0.5 rounded-full {pill.bg} {pill.text}">
        <span class="kanji">{pill.kanji}</span>
        <span class="uppercase tracking-wide">{pill.label}</span>
      </span>
      {@render chip(shareStateChip(item.state))}
      {#if deref}
        {@render chip({ bg: 'bg-paper-mute', text: 'text-ink-mute', label: deref })}
      {/if}
    </div>

    {#if item.title}
      <p class="text-sm text-ink m-0 leading-snug font-medium">{item.title}</p>
    {/if}
    {#if item.body}
      <p class="text-xs text-ink-soft mt-1 m-0 leading-snug line-clamp-3">{item.body}</p>
    {:else if held}
      <p class="text-xs text-ink-soft m-0 leading-snug">
        Content withheld — flagged for residual identifier risk.
      </p>
    {/if}

    <div class="flex flex-wrap items-center gap-x-4 gap-y-1 mt-2 text-xs text-ink-mute">
      <span data-attribution>{attributionSummary(item.attribution)}</span>
      {#if held}
        <span class="text-warning">won't ship this batch</span>
      {/if}
    </div>
  </article>
{/snippet}

<!-- After a publish: the honest "watch it travel" view — one row per item with
     the daemon's real outcome (published / held / queued / errored). -->
{#snippet travelBanner(outcome: PublishBatchOutcome)}
  <section
    data-outcome
    class="border border-paper-edge bg-paper-soft rounded-lg p-4 mb-6"
  >
    <div class="flex items-baseline gap-3 mb-2">
      <span class="kanji text-lg text-accent">旅</span>
      <h2 class="display text-lg font-normal m-0">Where your batch went</h2>
      <span class="font-mono text-xs text-ink-soft" data-outcome-summary>{outcomeSummary(outcome)}</span>
    </div>
    <div class="flex flex-col gap-2">
      {#each outcome.items as it, i (i)}
        <div
          class="flex flex-wrap items-center gap-2 text-xs"
          data-outcome-item
          data-result={it.result}
        >
          {@render chip(publishResultChip(it.result))}
          <span class="font-mono text-ink-mute">{it.remote_id ?? it.memory_id}</span>
          {#if it.message}
            <span class="text-danger">{it.message}</span>
          {/if}
        </div>
      {/each}
    </div>
  </section>
{/snippet}

<PageHeader
  variant="h2"
  eyebrow="Observatory · Share review"
  kanji="送"
  title="Ready to share."
  description="Lessons that generalised cleanly enough to leave this machine, scoped and attributed by origin. Review what's about to go, then publish. Held items stay back until the confidentiality gate clears — client work always leaves with its source dropped."
/>

<div class="max-w-[1060px] mx-auto px-7 pb-10" data-share-review>
  {#if loadError}
    <ScreenState status="error" error={loadError} {onretry} />
  {:else}
  {#if actions.error}
    <p class="text-xs text-danger mb-3" data-share-error>{actions.error}</p>
  {/if}

  {#if actions.outcome}
    {@render travelBanner(actions.outcome)}
  {/if}

  {#if !batch}
    {#if actions.outcome}
      <EmptyState
        kanji="送"
        title="batch sent"
        description="Your batch has left for the Dōjō. New lessons will queue here as they generalise cleanly enough to share."
      />
    {:else}
      <EmptyState
        kanji="送"
        title="nothing queued to share"
        description="Nothing is shareable by default. Lessons appear here once they've generalised past the confidence bar and an upstream batch is approved."
      />
    {/if}
  {:else}
    <!-- Destination / cadence bar — the batch header the spec calls for. -->
    <div
      class="flex flex-wrap items-center gap-3 bg-paper-soft border border-paper-edge rounded-lg px-4 py-3 mb-2"
      data-policy-bar
      data-batch={batch.batch_id}
    >
      <span class="kanji text-sm text-ink-mute">規</span>
      <span class="text-xs text-ink-soft">Destination</span>
      {#if unbound}
        <span class="font-mono text-xs px-2 py-0.5 rounded-full bg-warning-soft text-warning" data-unbound
          >unbound — nothing can leave</span
        >
      {:else}
        {#each dests as d, i (i)}
          <span
            class="inline-flex items-center gap-1 font-mono text-xs px-2 py-0.5 rounded-full bg-accent-soft text-accent"
            data-destination
          >
            <span class="kanji">結</span>{d}
          </span>
        {/each}
      {/if}
      <span class="font-mono text-xs px-2 py-0.5 rounded-full bg-paper-mute text-ink-mute" data-cadence
        >{batch.cadence}</span
      >
      {#if when}
        <span class="font-mono text-xs text-ink-faint" data-next-batch>next batch {when}</span>
      {/if}
      <span class="flex-1"></span>
      <span class="font-mono text-xs text-ink-soft" data-shippable-count>{nShip} to ship</span>
      {#if nHeld > 0}
        <span class="font-mono text-xs text-warning" data-held-count>{nHeld} held</span>
      {/if}
    </div>
    <p class="text-xs text-ink-soft mb-4 leading-normal">
      Your org's policy is the floor — an item can be stricter, never looser. Nothing leaves without
      passing through here.
    </p>

    <!-- Publish action — a large batch asks for confirmation first. -->
    <div class="flex flex-wrap items-center gap-2 mb-6">
      {#if confirming}
        <span class="text-xs text-ink-soft" data-confirm-prompt>
          Publish {nShip} items to your Dōjō? A maintainer approves before distribution.
        </span>
        <Button
          variant="primary"
          size="sm"
          data-action="confirm-publish"
          onclick={onPublish}
          disabled={actions.busy}
        >Confirm</Button>
        <Button
          variant="secondary"
          style="outline"
          size="sm"
          data-action="cancel-publish"
          onclick={() => (confirming = false)}
        >Cancel</Button>
      {:else}
        <Button
          variant="primary"
          size="sm"
          data-action="publish"
          onclick={onPublish}
          disabled={!canPublish}
        >
          <span class="kanji">共</span>{publishButtonLabel(nShip)}
        </Button>
      {/if}
    </div>

    <!-- Lane 1 · about to ship -->
    {#if grouped.shippable.length > 0}
      <section class="mb-8" data-lane="shippable">
        {#each grouped.shippable as item (item.memory_id)}
          {@render itemRow(item, false)}
        {/each}
      </section>
    {/if}

    <!-- Lane 2 · held back — shown, never publishable -->
    {#if grouped.held.length > 0}
      <section data-held-section>
        <div class="text-xs uppercase tracking-wide text-ink-faint font-semibold mb-2">
          Held — won't ship this batch
        </div>
        {#each grouped.held as item (item.memory_id)}
          {@render itemRow(item, true)}
        {/each}
      </section>
    {/if}
  {/if}
  {/if}
</div>
