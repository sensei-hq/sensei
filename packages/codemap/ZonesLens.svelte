<script lang="ts">
  // The Zones lens (M17): each component placed by instability (x) against
  // abstractness (y), with the main sequence, the zone of pain (stable and
  // concrete, bottom left) and of uselessness (abstract and unused, top right);
  // the furthest from the main sequence listed beside it.
  import { zonesView } from './lenses';
  import type { ZonePoint, Zones } from './types';

  let { data }: { data: Zones } = $props();
  const view = $derived(zonesView(data));
  const S = 360;
  const PAD = 32;
  const x = (p: ZonePoint) => PAD + p.instability * S;
  const y = (p: ZonePoint) => PAD + (1 - p.abstractness) * S;
  const tone = (z: ZonePoint['zone']) =>
    z === 'pain' ? 'var(--danger)' : z === 'useless' ? 'var(--warning)' : z === 'risk' ? 'var(--accent)' : 'var(--success)';
</script>

<div class="grid gap-4" style="grid-template-columns: auto 1fr;" data-testid="codemap-zones">
  <svg width={S + 2 * PAD} height={S + 2 * PAD} role="img" aria-label="Components by instability and abstractness">
    <rect x={PAD} y={PAD} width={S} height={S} fill="var(--paper-soft)" stroke="var(--paper-edge)" />
    <line x1={PAD} y1={PAD} x2={PAD + S} y2={PAD + S} stroke="var(--ink-faint)" stroke-dasharray="4 4" />
    <text x={PAD + 6} y={PAD + S - 8} class="text-xs" fill="var(--danger)">zone of pain</text>
    <text x={PAD + S - 6} y={PAD + 14} text-anchor="end" class="text-xs" fill="var(--warning)">zone of uselessness</text>
    <text x={PAD + S / 2} y={PAD + S + 22} text-anchor="middle" class="text-xs" fill="var(--ink-mute)">instability →</text>
    <text x="12" y={PAD + S / 2} transform={`rotate(-90 12 ${PAD + S / 2})`} text-anchor="middle" class="text-xs" fill="var(--ink-mute)">abstractness →</text>
    {#each view.placed as p (p.component)}
      <circle cx={x(p)} cy={y(p)} r="4" fill={tone(p.zone)} fill-opacity="0.7" data-testid={`zone-${p.component}`}>
        <title>{p.component} — I {p.instability.toFixed(2)}, A {p.abstractness.toFixed(2)}, D {p.distance.toFixed(2)}</title>
      </circle>
    {/each}
  </svg>
  <aside class="text-xs flex flex-col gap-2">
    <p class="m-0 text-ink-mute" data-testid="zones-summary">
      {`${view.placed.length} components placed · mean distance ${data.meanDistance.toFixed(2)} · pain ${view.byZone.pain} · useless ${view.byZone.useless} · risk ${view.byZone.risk} · main ${view.byZone.main}`}{view.unplaceable ? ` · ${view.unplaceable} without enough to place` : ''}
    </p>
    <h4 class="m-0 uppercase tracking-wide text-ink-mute">Furthest from the main sequence</h4>
    <ol class="m-0 pl-4 flex flex-col gap-1">
      {#each view.furthest as p (p.component)}
        <li class="font-mono"><span class="text-ink">{p.component}</span> <span class="text-ink-faint">{p.distance.toFixed(2)} · {p.zone}</span></li>
      {/each}
    </ol>
  </aside>
</div>
