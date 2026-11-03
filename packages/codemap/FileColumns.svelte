<script lang="ts">
  // L1 (M17): one module's files in role columns, and what it depends on and
  // is used by.
  import type { Column, Level1, Neighbour } from './types';

  let { data, columns, onopen, onmodule }: {
    data: Level1;
    columns: Column[];
    onopen: (id: string, path: string) => void;
    onmodule: (id: string) => void;
  } = $props();
</script>

{#snippet neighbours(title: string, list: Neighbour[])}
  <section>
    <h4 class="m-0 mb-1 uppercase tracking-wide text-ink-mute">{title} · {list.length}</h4>
    {#each list as n (n.module)}
      <button class="w-full flex justify-between bg-transparent border-none cursor-pointer p-1 rounded hover:bg-paper-mute font-mono text-xs"
        class:text-danger={n.direction === 'up'} onclick={() => onmodule(n.module)}>
        <span>{n.module}</span><span class="text-ink-faint">{n.weight}</span>
      </button>
    {:else}
      <p class="m-0 text-ink-faint">None.</p>
    {/each}
  </section>
{/snippet}

<div class="grid gap-4" style="grid-template-columns: 1fr 260px;" data-testid="codemap-files">
  <div class="grid gap-3" style={`grid-template-columns: repeat(${Math.max(columns.length, 1)}, minmax(0, 1fr));`}>
    {#each columns as c (c.role)}
      <section class="bg-paper-soft border border-paper-edge rounded-lg p-3" data-testid={`column-${c.role}`}>
        <h4 class="m-0 mb-2 text-xs uppercase tracking-wide text-ink-mute">{c.role} · {c.files.length + c.more}</h4>
        <ul class="m-0 p-0 list-none flex flex-col gap-1">
          {#each c.files as f (f.id)}
            <li>
              <button class="w-full text-left bg-transparent border-none cursor-pointer p-1 rounded hover:bg-paper-mute font-mono text-xs text-ink"
                onclick={() => onopen(f.id, f.path)} title={f.path} data-testid={`file-${f.path}`}>
                {f.path.split('/').slice(-1)[0]} <span class="text-ink-faint">{f.symbols}</span>
              </button>
            </li>
          {/each}
          {#if c.more}<li class="text-xs text-ink-faint p-1">+{c.more} more ({c.more_symbols} symbols)</li>{/if}
        </ul>
      </section>
    {:else}
      <p class="text-sm text-ink-soft">Every role in this module is turned off.</p>
    {/each}
  </div>
  <aside class="flex flex-col gap-3 text-xs">
    {@render neighbours('Depends on', data.dependsOn)}
    {@render neighbours('Used by', data.usedBy)}
  </aside>
</div>
