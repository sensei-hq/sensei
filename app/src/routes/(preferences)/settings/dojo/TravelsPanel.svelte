<script lang="ts">
  // What travels between sensei and the dōjō, in three directions. Presentational:
  // the list is `TRAVELS` (dojo-connection.ts).
  import { Card } from '@rokkit/ui';
  import type { TRAVELS } from './dojo-connection.js';

  let { travels }: { travels: typeof TRAVELS } = $props();
  const ARROW = { out: '→', both: '⇄', in: '←' } as const;
</script>

<section data-travels aria-labelledby="travels-h">
  <Card>
    {#snippet header()}
      <h2 id="travels-h" class="m-0 text-sm font-medium text-ink">What travels between sensei and the dōjō</h2>
    {/snippet}
    <div class="grid gap-3 md:grid-cols-3">
      {#each travels as t (t.direction)}
        <div data-direction={t.direction} class={['flex flex-col gap-1.5 rounded-md p-3', t.direction === 'in' ? 'bg-accent-soft' : 'bg-paper']}>
          <div class="flex items-center gap-1.5 text-xs font-medium text-on-accent-soft">
            <span aria-hidden="true">{ARROW[t.direction]}</span>{t.label}
          </div>
          <ul class="m-0 pl-4 flex flex-col gap-0.5 text-sm text-on-accent-soft">
            {#each t.items as item (item)}<li>{item}</li>{/each}
          </ul>
          {#if t.note}<p class="m-0 text-xs text-on-accent-soft">{t.note}</p>{/if}
        </div>
      {/each}
    </div>
    {#snippet footer()}
      <p class="m-0 text-xs text-ink-mute">Only repositories sensei has scanned on this machine can sync.</p>
    {/snippet}
  </Card>
</section>
