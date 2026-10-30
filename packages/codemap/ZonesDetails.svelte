<script lang="ts">
  // Zones beside its plot (M17): how many sit in each zone, and the components
  // furthest from the main sequence in the zone explored (filtered from the controls).
  import { zonesView } from './lenses';
  import type { ZonePoint, Zones } from './types';

  type Filter = 'all' | NonNullable<ZonePoint['zone']>;
  let { data, filter, onopen }: { data: Zones; filter: Filter; onopen: (id: string) => void } = $props();

  const view = $derived(zonesView(data, data.points.length));
  const furthest = $derived(view.furthest.filter((p) => filter === 'all' || p.zone === filter).slice(0, 10));
</script>

<section class="flex flex-col gap-3">
  <p class="m-0 text-sm text-ink-soft" data-testid="zones-summary">
    {`${view.placed.length} components placed · mean distance ${data.meanDistance.toFixed(2)} · pain ${view.byZone.pain} · useless ${view.byZone.useless} · risk ${view.byZone.risk} · main ${view.byZone.main}`}{view.unplaceable ? ` · ${view.unplaceable} without enough to place` : ''}
  </p>
  <div class="flex flex-col gap-1">
    <h4 class="m-0 text-xs uppercase tracking-wide text-ink-mute">Furthest from the main sequence</h4>
    <ol class="m-0 pl-5 flex flex-col gap-1" data-testid="zones-furthest">
      {#each furthest as p (p.component)}
        <li class="text-sm">
          <button class="p-0 bg-transparent border-none cursor-pointer text-left font-mono text-xs text-ink hover:underline break-all" onclick={() => onopen(p.component)}>{p.component}</button>
          <span class="text-xs text-ink-faint"> {p.distance.toFixed(2)} · {p.zone} · {p.ca} dependents</span>
        </li>
      {:else}
        <li class="text-sm text-ink-faint list-none -ml-5">None in this zone.</li>
      {/each}
    </ol>
  </div>
</section>
