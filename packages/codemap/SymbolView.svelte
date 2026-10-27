<script lang="ts">
  // L2 (M17): a file's symbols, with the symbols calling into it and those it
  // calls, at most seven a side.
  import type { Level2, SymbolRef } from './types';

  let { data }: { data: Level2 } = $props();
</script>

{#snippet side(title: string, list: SymbolRef[], total: number)}
  <section class="flex flex-col gap-1" data-testid={`symbols-${title.toLowerCase()}`}>
    <h4 class="m-0 mb-1 text-xs uppercase tracking-wide text-ink-mute">{title} · {total}</h4>
    {#each list as s (s.id)}
      <div class="bg-paper-soft border border-paper-edge rounded p-2 font-mono text-xs">
        {s.name} <span class="text-ink-faint">×{s.weight}</span>
        <div class="text-ink-faint truncate">{s.file ?? ''}</div>
      </div>
    {:else}
      <p class="m-0 text-xs text-ink-faint">None.</p>
    {/each}
    {#if total > list.length}<p class="m-0 text-xs text-ink-faint">+{total - list.length} more</p>{/if}
  </section>
{/snippet}

<div class="grid gap-4" style="grid-template-columns: 1fr 1.2fr 1fr;" data-testid="codemap-symbols">
  {@render side('Callers', data.callers, data.callersTotal)}
  <section class="bg-paper-soft border border-paper-edge rounded-lg p-3">
    <h4 class="m-0 mb-2 text-xs uppercase tracking-wide text-ink-mute">{data.path} · {data.symbols.length}</h4>
    <ul class="m-0 p-0 list-none flex flex-col gap-1 max-h-[480px] overflow-auto">
      {#each data.symbols as s (s.id)}
        <li class="font-mono text-xs text-ink">{s.name} <span class="text-ink-faint">{s.kind}{s.line ? ` :${s.line}` : ''}</span></li>
      {/each}
    </ul>
  </section>
  {@render side('Callees', data.callees, data.calleesTotal)}
</div>
