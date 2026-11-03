<script lang="ts">
  // One identity's dōjōs and the repositories each has filed. Presentational:
  // every line is decided in dojo-connection.ts.
  import { Card } from '@rokkit/ui';
  import { Notice } from '@sensei/ui';
  import { Kanji } from '$lib/components';
  import type { DojoGroup } from './dojo-connection.js';

  let { dojos }: { dojos: DojoGroup[] } = $props();
  const DOT = { syncing: 'bg-success', off: 'border border-ink-faint', failing: 'bg-danger' } as const;
</script>

<div class="flex flex-col gap-3">
  {#each dojos as d (d.id)}
    <section data-dojo={d.id} aria-label={d.name}>
      <Card>
        {#snippet header()}
          <div class="flex flex-wrap items-center gap-2.5">
            {#if d.kind !== 'unfiled'}<Kanji char={d.kanji} size="base" tone="accent" />{/if}
            <h3 class="m-0 text-sm font-medium text-ink">{d.name}</h3>
            {#if d.kind !== 'unfiled'}<span class="font-mono text-xs text-ink-mute border border-paper-edge rounded-full px-2">{d.kind}</span>{/if}
            {#if d.count !== null}<span class="ml-auto text-xs text-ink-mute" data-dojo-count>{d.count}</span>{/if}
          </div>
        {/snippet}
        {#if d.isOrg}
          <p class="m-0 mb-2 text-xs text-ink-mute">Sync for this organization's repositories is set by its admin in the dōjō. Everything is on unless they turn it off.</p>
        {/if}
        {#if d.empty}
          <p class="m-0 text-sm text-ink-mute" data-dojo-empty>Nothing in this view.</p>
        {:else if d.repos.length > 0}
          <ul class="m-0 p-0 list-none divide-y divide-paper-edge">
            {#each d.repos as r (r.key)}
              <li data-repository={r.key} data-state={r.state} class="py-2 flex flex-col gap-1">
                <div class="flex items-center gap-3">
                  <span class="text-sm text-ink truncate flex-1 min-w-0">{r.name}</span>
                  <span class="flex items-center gap-1.5 text-xs text-ink-mute whitespace-nowrap">
                    <span aria-hidden="true" class="inline-block w-2 h-2 rounded-full {DOT[r.category]}"></span>{r.status}
                  </span>
                </div>
                {#if r.notice}<Notice size="inline" bare {...r.notice} />{/if}
              </li>
            {/each}
          </ul>
        {/if}
      </Card>
    </section>
  {/each}
</div>
