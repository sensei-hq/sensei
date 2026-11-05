<script lang="ts">
  // One queue row (CONTRACT §1d): the title, the kind first then its tags, the
  // ticket button, and — in Triage only — `+` to add it to the backlog by its own
  // identity (an item's id, or a tracker task's ticket). Backlog rows are the same
  // row without `+`, in backlog order, with no rank numbers.
  import { providerLabel } from './relay-format.js';
  import type { RelayItemKind, RelayTicket } from '$lib/relay-api.js';

  let {
    id,
    title,
    kind,
    tags = [],
    sprint = null,
    addedByYou = false,
    ticket = null,
    onadd = null,
  }: {
    id: string;
    title: string;
    kind: RelayItemKind;
    tags?: string[];
    sprint?: string | null;
    addedByYou?: boolean;
    ticket?: RelayTicket | null;
    /** Given only in Triage: adds this row to the backlog. */
    onadd?: (() => Promise<string | null>) | null;
  } = $props();

  let busy = $state(false);

  const pills = $derived([
    kind,
    ...tags,
    ...(sprint ? [sprint] : []),
    ...(addedByYou ? ['added by you'] : []),
  ]);

  async function add(): Promise<void> {
    if (busy || !onadd) return;
    busy = true;
    await onadd();
    busy = false;
  }
</script>

<div class="border-t border-paper-edge py-2" data-queue-row={id}>
 <div class="flex flex-wrap items-center gap-2" data-row-line="1">
  <span class="text-sm" data-queue-title>{title}</span>
  {#each pills as pill, i (pill + i)}
    <span class="text-xs rounded-sm bg-paper-soft px-2 py-0.5 text-ink-mute" data-tag>{pill}</span>
  {/each}
  {#if ticket}
    <a
      class="ml-auto inline-flex items-center gap-1 text-xs underline"
      data-ticket
      href={ticket.url ?? undefined}
      target="_blank"
      rel="noopener noreferrer"
    >
      <span class="hidden sm:inline" data-ticket-provider>{providerLabel(ticket.provider)}</span>
      {ticket.key}
    </a>
  {/if}
  {#if onadd}
    <button
      type="button"
      class="rounded-sm border border-paper-edge px-2 py-0.5 text-xs disabled:opacity-60"
      data-add-to-backlog
      aria-label="Add to backlog: {title}"
      disabled={busy}
      onclick={() => void add()}
    >
      +
    </button>
  {/if}
 </div>
</div>
