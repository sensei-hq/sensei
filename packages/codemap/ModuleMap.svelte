<script lang="ts">
  // L0 on a canvas: group bands, blocks, weighted bands between them, hubs in
  // a side lane. Selection dims every band that does not touch it (M17).
  import { layoutLevel0 } from './layout';
  import type { Band, Level0 } from './types';

  let {
    data,
    bands,
    selected = null,
    cycle = null,
    dashed = false,
    onselect,
    onopen,
  }: {
    data: Level0;
    /** The bands this lens draws (all of them for Structure). */
    bands: Band[];
    selected?: string | null;
    /** Cycles lens: module → its cycle, to tint the members. */
    cycle?: Map<string, number> | null;
    /** Hidden coupling: the bands are co-change, not dependencies. */
    dashed?: boolean;
    onselect: (id: string | null) => void;
    onopen: (id: string) => void;
  } = $props();

  let width = $state(960);
  const layout = $derived(layoutLevel0({ ...data, bands }, { width }));
  const touches = (s: string, t: string) => !selected || s === selected || t === selected;
  const tone = (d: Band['direction']) =>
    d === 'up' ? 'var(--danger)' : d === 'skip' ? 'var(--warning)' : 'var(--ink-faint)';
</script>

<div class="w-full overflow-auto" bind:clientWidth={width} data-testid="codemap-modules">
  <svg {width} height={layout.height} role="img" aria-label="Modules and their dependencies">
    {#each layout.groups as g (g.id)}
      <rect x="4" y={g.y} width={layout.laneX - 8} height={g.height} rx="6" fill="var(--paper-soft)" />
      <text x="16" y={g.y + 14} class="text-xs" fill="var(--ink-mute)">{g.id}</text>
    {/each}
    {#if layout.hubLane.length}
      <text x={layout.laneX + 12} y="30" class="text-xs" fill="var(--ink-mute)">Hubs</text>
    {/if}
    {#each layout.bands as b (b.source + '>' + b.target)}
      <path
        d={b.path}
        fill="none"
        stroke={tone(b.direction)}
        stroke-width={b.width}
        stroke-opacity={touches(b.source, b.target) ? 0.85 : 0.08}
        stroke-dasharray={dashed ? '6 4' : undefined}
        data-testid={`band-${b.source}>${b.target}`}
      ><title>{b.source} → {b.target}: {b.weight}</title></path>
    {/each}
    {#each [...layout.blocks.values()] as p (p.id)}
      {@const folded = p.block.folded.length > 0}
      <g
        role="button"
        tabindex="0"
        class="cursor-pointer"
        onclick={() => onselect(selected === p.id ? null : p.id)}
        ondblclick={() => !folded && onopen(p.id)}
        onkeydown={(e) => e.key === 'Enter' && (folded ? onselect(p.id) : onopen(p.id))}
        data-testid={`block-${p.id}`}
      >
        <rect
          x={p.x}
          y={p.y}
          width={p.w}
          height={p.h}
          rx="4"
          fill={cycle?.has(p.id) ? 'var(--warning-soft)' : 'var(--paper)'}
          stroke={selected === p.id ? 'var(--accent)' : 'var(--paper-edge)'}
          stroke-width={selected === p.id ? 2 : 1}
          stroke-dasharray={folded ? '4 3' : undefined}
        />
        <text x={p.x + 8} y={p.y + 15} class="text-xs" fill="var(--ink)">
          {folded ? p.block.label : p.block.label.split('/').slice(-1)[0]}
        </text>
        <text x={p.x + 8} y={p.y + 29} class="text-xs" fill="var(--ink-mute)">
          {p.block.hub ? `${p.block.weight_in} in` : `${p.block.files} files`}
        </text>
        <title>{p.block.id} — {p.block.symbols} symbols, {p.block.files} files</title>
      </g>
    {/each}
  </svg>
</div>
