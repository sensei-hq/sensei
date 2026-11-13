<script lang="ts">
  // Relay · the task timeline (F8): one thin line per entry (time · actor ·
  // title), oldest first. Click expands to the full detail the entry renders
  // today (HistoryEntry, reused — not a second renderer). Instant toggle, no
  // motion to gate on prefers-reduced-motion.
  import type { RelayTaskView } from '$lib/relay-api.js';
  import { hhmm } from './relay-format.js';
  import HistoryEntry from './HistoryEntry.svelte';
  import { buildTimeline, foldReceipts } from './relay-timeline.js';

  let { view }: { view: RelayTaskView } = $props();

  const rows = $derived(buildTimeline(view));
  const receipts = $derived(foldReceipts(view.history));
  let open = $state<Record<string, boolean>>({});
</script>

<section class="flex flex-col gap-2" data-relay-timeline>
  <h3 class="text-xs uppercase tracking-wide text-ink-mute m-0">Timeline</h3>
  {#each rows as row (row.id)}
    <div class="rounded-sm border border-paper-edge" data-timeline-row={row.id}>
      <button
        class="flex w-full items-baseline gap-2 px-3 py-2 text-left cursor-pointer"
        data-timeline-toggle
        aria-expanded={open[row.id] ?? false}
        onclick={() => (open[row.id] = !(open[row.id] ?? false))}
      >
        <span class="text-xs text-ink-faint shrink-0">{hhmm(row.at)}</span>
        <span class="text-xs text-ink-mute shrink-0">{row.actor}</span>
        <span class="text-sm font-medium m-0">{row.title}</span>
      </button>
      {#if open[row.id]}
        <div class="px-3 pb-3" data-timeline-detail>
          {#if row.action}
            <HistoryEntry
              action={row.action}
              lanes={view.lanes}
              messageState={receipts[row.action.id] ?? (row.action.kind === 'message' ? 'sent' : null)}
            />
          {:else}
            <p class="text-sm text-ink-soft m-0">
              The coordinator cancelled this task. The broker records the cancel as the
              task's terminal state — there is no further detail.
            </p>
          {/if}
        </div>
      {/if}
    </div>
  {/each}
</section>
