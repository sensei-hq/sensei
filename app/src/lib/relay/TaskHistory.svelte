<script lang="ts">
  // A task's history, newest first, on rokkit's Timeline (CONTRACT §1e.6; COMPONENTS §3d):
  // one dot per entry, in its tone, on one line. Receipts are not entries of their own —
  // they fold into the message they reply to (F5).
  import { Timeline } from '@rokkit/ui';
  import type { RelayAction, RelayLaneRef } from '$lib/relay-api.js';
  import HistoryEntry from './HistoryEntry.svelte';
  import { foldReceipts, historyTone } from './relay-timeline.js';

  let { history, lanes }: { history: RelayAction[]; lanes: RelayLaneRef[] } = $props();

  const receipts = $derived(foldReceipts(history));
  const items = $derived(
    history
      .filter((a) => a.kind !== 'delivered' && a.kind !== 'read')
      .map((action) => ({ action, tone: historyTone(action) })),
  );
</script>

<section aria-label="History" class="flex flex-col gap-3" data-relay-section="history">
  <h3 class="m-0 text-xs font-medium uppercase tracking-[0.12em] text-ink-mute">History · newest first</h3>
  <!-- The dot sits on the card's title line: its border, padding and half the line, less half the dot. -->
  <Timeline {items} marker="dot" class="max-w-[920px] [--timeline-gap:0.75rem] [--timeline-dot-top:1.0625rem]">
    {#snippet content(item)}
      {@const action = item.action as RelayAction}
      <HistoryEntry {action} {lanes} messageState={receipts[action.id] ?? (action.kind === 'message' ? 'sent' : null)} />
    {/snippet}
  </Timeline>
</section>
