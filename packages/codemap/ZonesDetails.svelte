<script lang="ts">
  // Zones beside its plot (M17): how many sit in each zone — each a filter — and
  // the components furthest from the main sequence, in the zone explored.
  import { zonesView } from './lenses';
  import type { ZonePoint, Zones } from './types';

  type Filter = 'all' | NonNullable<ZonePoint['zone']>;
  let { data, filter, onfilter, onopen }: {
    data: Zones;
    filter: Filter;
    onfilter: (f: Filter) => void;
    onopen: (id: string) => void;
  } = $props();

  const view = $derived(zonesView(data, data.points.length));
  const ZONES: { id: Filter; label: string; tone: string }[] = [
    { id: 'all', label: 'All', tone: 'text-ink' },
    { id: 'pain', label: 'Pain', tone: 'text-danger' },
    { id: 'useless', label: 'Uselessness', tone: 'text-warning' },
    { id: 'risk', label: 'Risk', tone: 'text-accent' },
    { id: 'main', label: 'Main sequence', tone: 'text-success' },
  ];
  const count = (f: Filter) => (f === 'all' ? view.placed.length : view.byZone[f]);
  const furthest = $derived(view.furthest.filter((p) => filter === 'all' || p.zone === filter).slice(0, 10));
</script>

<section class="flex flex-col gap-3">
  <p class="m-0 text-sm text-ink-soft" data-testid="zones-summary">
    {`${view.placed.length} components placed · mean distance ${data.meanDistance.toFixed(2)} · pain ${view.byZone.pain} · useless ${view.byZone.useless} · risk ${view.byZone.risk} · main ${view.byZone.main}`}{view.unplaceable ? ` · ${view.unplaceable} without enough to place` : ''}
  </p>
  <div class="flex flex-wrap gap-1" role="group" aria-label="Zone">
    {#each ZONES as z (z.id)}
      <button class={`px-2 py-1 text-xs rounded border border-paper-edge cursor-pointer ${z.tone}`}
        class:bg-paper-mute={filter === z.id} class:bg-transparent={filter !== z.id}
        aria-pressed={filter === z.id} onclick={() => onfilter(z.id)} data-testid={`zones-filter-${z.id}`}>{z.label} · {count(z.id)}</button>
    {/each}
  </div>
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
