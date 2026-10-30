<script lang="ts">
  // L0 on a canvas: group bands, blocks, weighted bands between them, hubs in
  // a side lane (M17). A click opens a module; a fold or a hub is selected for
  // the inspector. Hovering or selecting dims every band that does not touch it.
  import { layoutLevel0 } from './layout';
  import { bandTone, complexityTier, shownFiles } from './view';
  import type { Band, Block, Level0, Role } from './types';

  let {
    data,
    bands,
    selected = null,
    cycle = null,
    dashed = false,
    complexity = false,
    byDirection = false,
    roles,
    zoom = 1,
    onselect,
    onopen,
  }: {
    data: Level0;
    /** The bands this lens draws, after the band filters. */
    bands: Band[];
    selected?: string | null;
    /** Cycles lens: module → its cycle, to tint the members. */
    cycle?: Map<string, number> | null;
    /** Hidden coupling: the bands are co-change, not dependencies. */
    dashed?: boolean;
    /** Complexity lens: tint blocks by their share over cyclomatic 10. */
    complexity?: boolean;
    /** Tint every band by direction, not only upward. */
    byDirection?: boolean;
    /** The roles shown: a block's file count is theirs. */
    roles: Role[];
    /** Over fitting the drawing to the column; 1 is fit. */
    zoom?: number;
    onselect: (id: string | null) => void;
    onopen: (id: string) => void;
  } = $props();

  /** The width the drawing is laid out at; Fit scales it to the column. */
  const DRAW_W = 1280;
  /** Fit never shrinks below this; past it the canvas scrolls. */
  const FIT_FLOOR = 0.6;

  let column = $state(0);
  let hovered = $state<string | null>(null);
  const focus = $derived(hovered ?? selected);
  const layout = $derived(layoutLevel0({ ...data, bands }, { width: DRAW_W }));
  const fit = $derived(column <= 0 ? 1 : Math.max(FIT_FLOOR, Math.min(1, column / DRAW_W)));
  const scale = $derived(fit * zoom);
  const touches = (s: string, t: string) => !focus || s === focus || t === focus;

  const TIER_FILL = { danger: 'var(--danger-soft)', warning: 'var(--warning-soft)', neutral: 'var(--paper)', unmeasured: 'var(--paper-mute)' };
  const fillOf = (b: Block) =>
    complexity ? TIER_FILL[complexityTier(b).tone] : cycle?.has(b.id) ? 'var(--warning-soft)' : 'var(--paper)';
  const noteOf = (b: Block) =>
    complexity ? complexityTier(b).label : b.hub ? `${b.weight_in} in` : `${shownFiles(b, roles)} files`;
  const pick = (b: Block) =>
    b.folded.length || b.hub ? onselect(selected === b.id ? null : b.id) : onopen(b.id);
</script>

<!-- A scrollable region must take focus for keyboard users (axe scrollable-region-focusable). -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div class="w-full overflow-auto" tabindex="0" role="region" aria-label="Modules and their dependencies, scrollable" bind:clientWidth={column} data-testid="codemap-modules" data-zoom={zoom}>
  <svg
    width={Math.round(layout.width * scale)}
    height={Math.round(layout.height * scale)}
    viewBox={`0 0 ${layout.width} ${layout.height}`}
    role="img"
    aria-label="Modules and their dependencies"
  >
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
        stroke={bandTone(b.direction, byDirection)}
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
        onclick={() => pick(p.block)}
        onmouseenter={() => (hovered = p.id)}
        onmouseleave={() => (hovered = null)}
        onkeydown={(e) => e.key === 'Enter' && pick(p.block)}
        data-testid={`block-${p.id}`}
      >
        <rect
          x={p.x}
          y={p.y}
          width={p.w}
          height={p.h}
          rx="4"
          fill={fillOf(p.block)}
          stroke={selected === p.id ? 'var(--accent)' : 'var(--paper-edge)'}
          stroke-width={selected === p.id ? 2 : 1}
          stroke-dasharray={folded ? '4 3' : undefined}
        />
        <text x={p.x + 8} y={p.y + 15} class="text-xs" fill="var(--ink)">
          {folded ? p.block.label : p.block.label.split('/').slice(-1)[0]}
        </text>
        <text x={p.x + 8} y={p.y + 29} class="text-xs" fill="var(--ink-mute)" data-testid={`block-note-${p.id}`}>
          {noteOf(p.block)}
        </text>
        <title>{p.block.id} — {p.block.symbols} symbols, {shownFiles(p.block, roles)} files</title>
      </g>
    {/each}
  </svg>
</div>
