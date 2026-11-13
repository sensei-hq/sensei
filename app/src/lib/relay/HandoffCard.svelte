<script lang="ts">
  // Handed off (COMPONENTS §3d HandoffCard; CONTRACT §1e.5): when, the brief the lane
  // was given, the docs it points at as chips. Radius 12 (#388).
  import type { RelayHandoffView } from '@sensei/state';

  let { handoff }: { handoff: RelayHandoffView } = $props();
</script>

<section data-handoff class="flex flex-col gap-2 py-3 px-3.5 rounded-xl border border-solid border-paper-edge bg-paper max-w-[920px]">
  <p data-handoff-label class="m-0 text-xs uppercase tracking-[0.12em] text-ink-mute">{handoff.label}</p>
  <p data-handoff-brief class="m-0 text-base text-ink">{handoff.brief}</p>
  {#if handoff.docs.length}
    <div class="flex flex-wrap gap-2">
      {#each handoff.docs as doc (doc)}
        <code data-handoff-doc class="py-1 px-2.5 rounded-lg border border-solid border-paper-edge bg-paper font-mono text-xs">{doc}</code>
      {/each}
    </div>
  {/if}
</section>
