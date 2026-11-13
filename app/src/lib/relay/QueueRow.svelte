<script lang="ts">
  // One queue row (COMPONENTS §3c; CONTRACT §1d): the title over the kind pill (in
  // its tone) and the tag pills; the ticket as a pill linking out (provider and key —
  // the key alone on a phone); + in Triage to add it to the backlog by its own
  // identity; in the backlog a checkbox picks it for a run. It draws toQueueRows'
  // data and calls back.
  import type { RelayQueueRowView } from '@sensei/state';
  import Pill from './Pill.svelte';
  import Tag from './Tag.svelte';
  import { NOW_CARD_ROW } from './primitives.js';

  let {
    row,
    onadd = null,
    selected = false,
    ontoggle = null,
  }: {
    row: RelayQueueRowView;
    /** Triage: adds this row to the backlog; its refusal, or null. */
    onadd?: (() => Promise<string | null>) | null;
    /** Backlog: picked for a run. */
    selected?: boolean;
    ontoggle?: (() => void) | null;
  } = $props();

  let busy = $state(false);

  async function add(): Promise<void> {
    if (busy || !onadd) return;
    busy = true;
    await onadd();
    busy = false;
  }
</script>

<li
  data-queue-row={row.id}
  data-selected={row.selectable ? selected : undefined}
  class="{NOW_CARD_ROW} flex flex-wrap items-center gap-x-3 gap-y-2 py-2.5 px-3.5"
>
  {#if row.selectable}
    <input
      type="checkbox"
      data-relay-select
      aria-label="Pick for a run: {row.title}"
      checked={selected}
      onchange={() => ontoggle?.()}
      class="self-start mt-1 w-3.5 h-3.5 accent-accent"
    />
  {/if}
  <div class="flex-[1_1_calc(100%-150px)] md:flex-[1_1_260px] min-w-0 flex flex-col gap-1.5">
    <span data-queue-title class="text-base text-ink">{row.title}</span>
    <span data-queue-pills class="flex flex-wrap items-center gap-1.5">
      <span><Pill label={row.kind.label} tone={row.kind.tone} /></span>
      {#each row.tags as tag (tag)}
        <span><Tag kind="family" label={tag} /></span>
      {/each}
    </span>
  </div>
  {#if row.ticket}
    <a
      data-ticket
      href={row.ticket.url ?? undefined}
      target="_blank"
      rel="noopener noreferrer"
      class="inline-flex items-center gap-1.5 min-h-8 px-3 rounded-lg border border-solid border-paper-edge bg-paper text-sm text-ink no-underline"
    >
      <span data-ticket-provider class="hidden md:inline text-ink-soft">{row.ticket.provider}</span>
      <span data-ticket-key class="font-mono">{row.ticket.key}</span>
      <span aria-hidden="true" class="i-solar-arrow-right-up-linear text-sm text-ink-soft"></span>
    </a>
  {/if}
  {#if row.canAdd && onadd}
    <button
      type="button"
      data-add-to-backlog
      aria-label="Add to backlog: {row.title}"
      disabled={busy}
      onclick={() => void add()}
      class="inline-flex items-center justify-center w-8 h-8 rounded-lg border border-solid border-paper-edge bg-paper text-ink cursor-pointer disabled:opacity-60"
    ><span aria-hidden="true" class="text-lg leading-none">+</span></button>
  {/if}
</li>
