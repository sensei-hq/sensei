<script lang="ts">
  // The Zones lens (M17): each component placed by instability (x) against
  // abstractness (y), with the main sequence, the zone of pain (stable and
  // concrete, bottom left) and of uselessness (abstract and unused, top right).
  // The frame draws at once and the dots arrive into it; a zone filter dims the
  // rest rather than dropping them. A dot opens its module.
  import { zonesView } from './lenses';
  import type { ZonePoint, Zones } from './types';

  let { data, filter = 'all', zoom = 1, onopen }: {
    data: Zones | null;
    /** Over fitting the plot to the column; 1 is fit. */
    zoom?: number;
    filter?: 'all' | ZonePoint['zone'];
    onopen: (id: string) => void;
  } = $props();

  let column = $state(480);
  const PAD = 40;
  const S = $derived(Math.max(280, Math.min(column - 2 * PAD, 640)));
  const placed = $derived(data ? zonesView(data).placed : []);
  const x = (p: ZonePoint) => PAD + p.instability * S;
  const y = (p: ZonePoint) => PAD + (1 - p.abstractness) * S;
  const tone = (z: ZonePoint['zone']) =>
    z === 'pain' ? 'var(--danger)' : z === 'useless' ? 'var(--warning)' : z === 'risk' ? 'var(--accent)' : 'var(--success)';
  const shown = (p: ZonePoint) => filter === 'all' || p.zone === filter;
</script>

<!-- A scrollable region must take focus for keyboard users (axe scrollable-region-focusable). -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div class="w-full overflow-auto" tabindex="0" role="region" aria-label="Components by instability and abstractness, scrollable" bind:clientWidth={column} data-testid="codemap-zones">
  <svg width={(S + 2 * PAD) * zoom} height={(S + 2 * PAD) * zoom} viewBox={`0 0 ${S + 2 * PAD} ${S + 2 * PAD}`} role="img" aria-label="Components by instability and abstractness">
    <rect x={PAD} y={PAD} width={S} height={S} fill="var(--paper-soft)" stroke="var(--paper-edge)" />
    <line x1={PAD} y1={PAD} x2={PAD + S} y2={PAD + S} stroke="var(--ink-faint)" stroke-dasharray="4 4" />
    <text x={PAD + 8} y={PAD + S - 10} class="text-xs" fill="var(--danger)">zone of pain</text>
    <text x={PAD + S - 8} y={PAD + 16} text-anchor="end" class="text-xs" fill="var(--warning)">zone of uselessness</text>
    <text x={PAD + S / 2} y={PAD + S + 26} text-anchor="middle" class="text-xs" fill="var(--ink-mute)">instability →</text>
    <text x="14" y={PAD + S / 2} transform={`rotate(-90 14 ${PAD + S / 2})`} text-anchor="middle" class="text-xs" fill="var(--ink-mute)">abstractness →</text>
    {#if !data}
      <text x={PAD + S / 2} y={PAD + S / 2} text-anchor="middle" class="text-xs" fill="var(--ink-faint)">Placing components…</text>
    {/if}
    {#each placed as p (p.component)}
      <circle cx={x(p)} cy={y(p)} r="5" fill={tone(p.zone)} fill-opacity={shown(p) ? 0.8 : 0.12}
        role="button" tabindex="0" class="cursor-pointer"
        onclick={() => onopen(p.component)} onkeydown={(e) => e.key === 'Enter' && onopen(p.component)}
        data-testid={`zone-${p.component}`}>
        <title>{p.component} — I {p.instability.toFixed(2)}, A {p.abstractness.toFixed(2)}, distance {p.distance.toFixed(2)}, {p.ca} dependents</title>
      </circle>
    {/each}
  </svg>
</div>
