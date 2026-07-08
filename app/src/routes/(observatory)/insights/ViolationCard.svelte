<script lang="ts">
  import type { ViolationCardVm } from './insights-board.svelte.js';

  let { memory }: { memory: ViolationCardVm } = $props();
</script>

<!--
  A memory contradicted by recent work (violated_count > 0) — the highest
  signal in Now. Display-only: memory write-actions (reinforce / challenge /
  archive) are deferred for this screen, so this reads as an alert, not a form.
-->
<article
  data-component="violation-card"
  data-memory-id={memory.id}
  class="rounded bg-accent-soft border border-transparent border-l-2 border-l-accent py-2 px-3"
>
  <div class="flex items-center gap-1 mb-1">
    <span class="text-xs uppercase tracking-wider text-accent font-medium" data-violation-count>
      violated · {memory.violatedCount}×
    </span>
  </div>
  <p class="text-sm text-ink m-0 leading-snug font-medium">{memory.title}</p>
  {#if memory.content?.trim()}
    <p class="text-xs text-ink-soft mt-1 m-0 leading-normal line-clamp-2">{memory.content}</p>
  {/if}
</article>
