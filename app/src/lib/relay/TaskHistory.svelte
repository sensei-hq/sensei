<script lang="ts">
  // A task's history, newest first, on its timeline (CONTRACT §1e.6; COMPONENTS §3d): a
  // 2px rule each entry's node sits on. Receipts are not entries of their own — they fold
  // into the message they reply to (F5).
  import type { RelayAction, RelayLaneRef } from '$lib/relay-api.js';
  import HistoryEntry from './HistoryEntry.svelte';

  let { history, lanes }: { history: RelayAction[]; lanes: RelayLaneRef[] } = $props();

  const receipts = $derived.by(() => {
    const out: Record<string, 'sent' | 'delivered' | 'read'> = {};
    for (const a of history) {
      if (!a.reply_to) continue;
      if (a.kind === 'read') out[a.reply_to] = 'read';
      else if (a.kind === 'delivered' && out[a.reply_to] !== 'read') out[a.reply_to] = 'delivered';
    }
    return out;
  });
  const entries = $derived(history.filter((a) => a.kind !== 'delivered' && a.kind !== 'read'));
</script>

<section aria-label="History" class="flex flex-col gap-3" data-relay-section="history">
  <h3 class="m-0 text-xs font-medium uppercase tracking-[0.12em] text-ink-mute">History · newest first</h3>
  <div class="flex flex-col gap-3 ml-1.5 pl-6 border-l-2 border-solid border-y-0 border-r-0 border-paper-edge max-w-[920px]">
    {#each entries as action (action.id)}
      <HistoryEntry {action} {lanes} messageState={receipts[action.id] ?? (action.kind === 'message' ? 'sent' : null)} />
    {/each}
  </div>
</section>
