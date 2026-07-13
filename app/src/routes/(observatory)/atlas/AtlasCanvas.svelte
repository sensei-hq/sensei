<script lang="ts">
  import {
    forceSimulation,
    forceLink,
    forceManyBody,
    forceCenter,
    forceCollide,
    forceX,
    forceY,
    type Simulation,
    type SimulationNodeDatum,
    type SimulationLinkDatum,
  } from 'd3-force';
  import { select } from 'd3-selection';
  import { zoom, type ZoomBehavior } from 'd3-zoom';
  import { neighbourIds, type AtlasLink, type RenderNode } from './atlas-graph.svelte.js';

  let {
    nodes,
    links,
    selectedId = null,
    onselect,
  }: {
    nodes: RenderNode[];
    links: AtlasLink[];
    selectedId?: string | null;
    onselect: (id: string) => void;
  } = $props();

  const WIDTH = 820;
  const HEIGHT = 560;

  interface SimNode extends SimulationNodeDatum, RenderNode {}
  interface Placed extends RenderNode {
    x: number;
    y: number;
  }

  let svgEl = $state<SVGSVGElement | undefined>();
  let transform = $state('translate(0,0) scale(1)');

  // Settle the force layout synchronously — the sim is a pure, deterministic
  // function of the nodes/links, so it recomputes as a `$derived` (no per-frame
  // animation, no leftover simulation to tear down).
  const placed = $derived.by<Placed[]>(() => {
    if (nodes.length === 0) return [];

    const simNodes: SimNode[] = nodes.map((n, i) => {
      const a = (i / Math.max(1, nodes.length)) * Math.PI * 2;
      const ring = 120 + (i % 7) * 26;
      return { ...n, x: WIDTH / 2 + Math.cos(a) * ring, y: HEIGHT / 2 + Math.sin(a) * ring };
    });
    const simLinks: SimulationLinkDatum<SimNode>[] = links.map((l) => ({
      source: l.source,
      target: l.target,
    }));

    const sim: Simulation<SimNode, SimulationLinkDatum<SimNode>> = forceSimulation(simNodes)
      .force(
        'link',
        forceLink<SimNode, SimulationLinkDatum<SimNode>>(simLinks)
          .id((d) => d.id)
          .distance(70)
          .strength(0.25),
      )
      .force('charge', forceManyBody().strength(-160))
      .force('center', forceCenter(WIDTH / 2, HEIGHT / 2))
      .force('collide', forceCollide<SimNode>().radius((d) => d.r + 4))
      .force('x', forceX(WIDTH / 2).strength(0.035))
      .force('y', forceY(HEIGHT / 2).strength(0.035))
      .stop();

    const ticks = Math.min(400, 120 + simNodes.length);
    for (let i = 0; i < ticks; i++) sim.tick();

    return simNodes.map((n) => ({
      id: n.id,
      label: n.label,
      kind: n.kind,
      r: n.r,
      color: n.color,
      ring: n.ring,
      x: n.x ?? WIDTH / 2,
      y: n.y ?? HEIGHT / 2,
    }));
  });

  const posById = $derived(new Map(placed.map((p) => [p.id, p])));
  const highlight = $derived(neighbourIds(links, selectedId));

  // Labels stay hidden in the dense symbol view (they'd overlap into mush) and
  // show in the sparse community view or for whatever the user has focused.
  const showAllLabels = $derived(nodes.length <= 90);
  function labelVisible(id: string): boolean {
    return showAllLabels || selectedId == null || highlight.has(id);
  }
  function truncate(label: string): string {
    return label.length > 20 ? label.slice(0, 19) + '…' : label;
  }

  function nodeOpacity(id: string): number {
    if (selectedId == null) return 1;
    return highlight.has(id) ? 1 : 0.18;
  }
  function edgeIncident(l: AtlasLink): boolean {
    return selectedId != null && (l.source === selectedId || l.target === selectedId);
  }

  // ── Zoom / pan ──────────────────────────────────────────────────────────────
  // Event-driven, so this stays an $effect: d3-zoom pushes new transforms.
  $effect(() => {
    const el = svgEl;
    if (!el) return;
    const sel = select(el);
    const behavior: ZoomBehavior<SVGSVGElement, unknown> = zoom<SVGSVGElement, unknown>()
      .scaleExtent([0.25, 5])
      .on('zoom', (event) => {
        transform = event.transform.toString();
      });
    sel.call(behavior);
    return () => {
      sel.on('.zoom', null);
    };
  });
</script>

<svg
  bind:this={svgEl}
  viewBox="0 0 {WIDTH} {HEIGHT}"
  preserveAspectRatio="xMidYMid meet"
  class="w-full h-full block select-none cursor-grab"
  role="presentation"
  data-component="atlas-canvas"
>
  <defs>
    <marker id="atlas-arrow" markerWidth="8" markerHeight="8" refX="6.5" refY="4" orient="auto">
      <path d="M0,0 L8,4 L0,8 z" fill="var(--ink-faint)" />
    </marker>
  </defs>

  <g {transform}>
    <!-- edges -->
    {#each links as l, i (i)}
      {@const a = posById.get(l.source)}
      {@const b = posById.get(l.target)}
      {#if a && b}
        <line
          x1={a.x}
          y1={a.y}
          x2={b.x}
          y2={b.y}
          stroke={edgeIncident(l) ? 'var(--accent)' : 'var(--ink-faint)'}
          stroke-width={edgeIncident(l) ? 1.6 : 1}
          opacity={selectedId == null ? 0.35 : edgeIncident(l) ? 0.9 : 0.08}
          marker-end="url(#atlas-arrow)"
        />
      {/if}
    {/each}

    <!-- nodes -->
    {#each placed as n (n.id)}
      <g
        opacity={nodeOpacity(n.id)}
        class="cursor-pointer"
        role="button"
        tabindex="-1"
        aria-label={n.label}
        onclick={() => onselect(n.id)}
        onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && onselect(n.id)}
      >
        {#if n.id === selectedId}
          <circle cx={n.x} cy={n.y} r={n.r + 5} fill="none" stroke="var(--accent)" stroke-width="2" />
        {/if}
        <circle
          cx={n.x}
          cy={n.y}
          r={n.r}
          fill={n.color}
          stroke={n.ring ? 'var(--ink-faint)' : 'none'}
          stroke-width={n.ring ? 1 : 0}
        />
        {#if labelVisible(n.id)}
          <text
            x={n.x}
            y={n.y + n.r + 11}
            text-anchor="middle"
            class="font-mono"
            style="font-size: 9px; fill: var(--ink-soft);"
          >{truncate(n.label)}</text>
        {/if}
      </g>
    {/each}
  </g>
</svg>
