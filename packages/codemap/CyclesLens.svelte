<script lang="ts">
  // The Cycles lens (M17): every cycle on its own — its members round a ring, the
  // dependencies that close it, the lightest one dashed as the cheapest to cut —
  // and nothing else, so what it is about cannot be lost among the rest. A member
  // opens its module; the drawing follows the zoom.
  import { cycleRings } from './cycles';
  import type { Level0 } from './types';

  let { data, zoom = 1, onfocus }: { data: Level0; zoom?: number; onfocus: (id: string) => void } = $props();
  const rings = $derived(cycleRings(data));
  const short = (id: string) => id.split('/').slice(-1)[0];
  const cutOf = (r: (typeof rings)[number]) => r.edges.find((e) => e.cut);
</script>

<!-- A scrollable region must take focus for keyboard users (axe scrollable-region-focusable). -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div class="w-full overflow-auto" tabindex="0" role="region" aria-label="Cycles, scrollable" data-testid="codemap-cycles">
  {#if rings.length}
    <div class="flex flex-wrap gap-4">
      {#each rings as r, i (i)}
        {@const cut = cutOf(r)}
        <figure class="m-0 flex flex-col gap-2 p-3 rounded-lg border border-paper-edge" data-cycle>
          <figcaption class="text-xs text-ink-mute" data-cycle-title>
            Cycle {i + 1} · {r.nodes.length} modules{cut ? ` · cut ${cut.source} → ${cut.target}` : ''}
          </figcaption>
          <svg width={r.size * zoom} height={r.size * zoom} viewBox={`0 0 ${r.size} ${r.size}`} role="img"
            aria-label={`Cycle ${i + 1}: ${r.nodes.map((n) => n.id).join(', ')}`}>
            <defs>
              <marker id={`cyc-arrow-${i}`} viewBox="0 0 8 8" refX="7" refY="4" markerWidth="7" markerHeight="7" orient="auto">
                <path d="M0 0 L8 4 L0 8 z" fill="var(--warning)" />
              </marker>
              <marker id={`cyc-cut-${i}`} viewBox="0 0 8 8" refX="7" refY="4" markerWidth="7" markerHeight="7" orient="auto">
                <path d="M0 0 L8 4 L0 8 z" fill="var(--danger)" />
              </marker>
            </defs>
            {#each r.edges as e (`${e.source}>${e.target}`)}
              <path d={e.path} fill="none" stroke={e.cut ? 'var(--danger)' : 'var(--warning)'}
                stroke-width={e.cut ? 2 : 1.5} stroke-dasharray={e.cut ? '5 4' : undefined}
                marker-end={`url(#cyc-${e.cut ? 'cut' : 'arrow'}-${i})`}
                data-edge={`${e.source}>${e.target}`} data-cut={e.cut || undefined}>
                <title>{e.source} → {e.target}: {e.weight}{e.cut ? ' — the lightest: cut this' : ''}</title>
              </path>
              <text x={e.label.x} y={e.label.y} text-anchor="middle" class="text-xs" fill={e.cut ? 'var(--danger)' : 'var(--ink-mute)'}>{e.weight}</text>
            {/each}
            {#each r.nodes as n (n.id)}
              {@const outside = n.x >= r.size / 2}
              <g role="button" tabindex="0" class="cursor-pointer" data-cycle-node={n.id}
                onclick={() => onfocus(n.id)} onkeydown={(ev) => ev.key === 'Enter' && onfocus(n.id)}>
                <circle cx={n.x} cy={n.y} r="6" fill="var(--paper)" stroke="var(--warning)" stroke-width="2" />
                <text x={n.x + (outside ? 12 : -12)} y={n.y + 4} text-anchor={outside ? 'start' : 'end'} class="text-xs" fill="var(--ink)">{short(n.id)}</text>
                <title>{n.id}</title>
              </g>
            {/each}
          </svg>
        </figure>
      {/each}
    </div>
  {:else}
    <p class="m-0 text-sm text-success">No cycles: every module can be layered and released apart.</p>
  {/if}
</div>
