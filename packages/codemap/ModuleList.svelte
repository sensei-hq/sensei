<script lang="ts">
  // L0 as a list, for a narrow screen (M17, under 640px): modules grouped,
  // with their weights and badges; rows at least 44px tall.
  import { groupOrder } from './layout';
  import type { Level0 } from './types';

  let { data, onopen }: { data: Level0; onopen: (id: string) => void } = $props();
  const groups = $derived(groupOrder(data.blocks.filter((b) => !b.hub), data.groupBy));
  const upward = $derived(new Set(data.bands.filter((b) => b.direction === 'up').map((b) => b.source)));
</script>

<div class="flex flex-col gap-3" data-testid="codemap-list">
  {#each groups as g (g)}
    <section>
      <h4 class="m-0 mb-1 text-xs uppercase tracking-wide text-ink-mute">{g}</h4>
      {#each data.blocks.filter((b) => b.group === g && !b.hub) as b (b.id)}
        <button class="w-full min-h-11 flex items-center justify-between gap-2 px-3 bg-paper-soft border border-paper-edge rounded mb-1 text-left cursor-pointer"
          disabled={b.folded.length > 0} onclick={() => onopen(b.id)}>
          <span class="font-mono text-xs text-ink truncate">{b.label}</span>
          <span class="text-xs text-ink-faint">{b.files} files{upward.has(b.id) ? ' · upward' : ''}</span>
        </button>
      {/each}
    </section>
  {/each}
</div>
