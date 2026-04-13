<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import * as d3Force from 'd3-force';
  import * as d3Selection from 'd3-selection';
  import * as d3Zoom from 'd3-zoom';

  interface GraphNode {
    id: string;
    name: string;
    kind: string;
    file: string;
    line: number;
    complexity?: number;
    // d3 simulation adds these
    x?: number;
    y?: number;
    vx?: number;
    vy?: number;
    fx?: number | null;
    fy?: number | null;
  }

  interface GraphEdge {
    source: string | GraphNode;
    target: string | GraphNode;
    type: string;
  }

  let { nodes = [], edges = [], onSelectNode }:
    { nodes: GraphNode[]; edges: GraphEdge[]; onSelectNode?: (node: GraphNode | null) => void } = $props();

  let container: HTMLDivElement;
  let canvas: HTMLCanvasElement;
  let simulation: d3Force.Simulation<GraphNode, GraphEdge> | null = null;
  let transform = d3Zoom.zoomIdentity;
  let selectedNode = $state<GraphNode | null>(null);
  let hoveredNode = $state<GraphNode | null>(null);
  let width = $state(800);
  let height = $state(500);

  // Color by node kind (semantic)
  const KIND_COLORS: Record<string, string> = {
    'function': '#6366f1', // indigo — functions
    'method':   '#6366f1',
    'class':    '#f59e0b', // amber — classes/types
    'struct':   '#f59e0b',
    'interface':'#f59e0b',
    'type':     '#f59e0b',
    'enum':     '#f59e0b',
    'file':     '#10b981', // emerald — files
    'doc':      '#06b6d4', // cyan — docs
    'component':'#ec4899', // pink — components
    'hook':     '#ec4899',
    'const':    '#94a3b8', // slate — constants
  };

  function nodeColor(node: GraphNode): string {
    return KIND_COLORS[node.kind] ?? '#94a3b8';
  }

  function nodeRadius(node: GraphNode): number {
    if (isLargeGraph) return 2; // Small dots for large graphs
    const degree = edges.filter(e => {
      const src = typeof e.source === 'string' ? e.source : e.source.id;
      const tgt = typeof e.target === 'string' ? e.target : e.target.id;
      return src === node.id || tgt === node.id;
    }).length;
    return Math.max(3, Math.min(12, 3 + degree * 0.8));
  }

  // Large graph mode: >200 nodes → smaller dots, no labels, tighter simulation
  let isLargeGraph = $derived(nodes.length > 200);

  function setupSimulation() {
    if (!canvas || nodes.length === 0) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    // Clone data for d3 (it mutates)
    const simNodes = nodes.map(n => ({ ...n }));
    // Filter edges: only keep edges where both source and target exist in nodes
    const nodeIds = new Set(simNodes.map(n => n.id));
    const simEdges = edges
      .map(e => ({ ...e }))
      .filter(e => {
        const srcId = typeof e.source === 'string' ? e.source : e.source.id;
        const tgtId = typeof e.target === 'string' ? e.target : e.target.id;
        return nodeIds.has(srcId) && nodeIds.has(tgtId);
      });

    const large = simNodes.length > 200;
    const charge = large ? -20 : -80;
    const linkDist = large ? 30 : 60;
    const linkStr = large ? 0.1 : 0.3;

    simulation = d3Force.forceSimulation(simNodes)
      .force('link', d3Force.forceLink<GraphNode, GraphEdge>(simEdges).id((d: GraphNode) => d.id).distance(linkDist).strength(linkStr))
      .force('charge', d3Force.forceManyBody().strength(charge))
      .force('center', d3Force.forceCenter(width / 2, height / 2))
      .force('collision', d3Force.forceCollide().radius(large ? 3 : 8))
      .on('tick', () => draw(ctx, simNodes, simEdges));

    // Zoom
    const sel = d3Selection.select(canvas);
    const zoom = d3Zoom.zoom<HTMLCanvasElement, unknown>()
      .scaleExtent([0.1, 5])
      .on('zoom', (event: any) => {
        transform = event.transform;
        draw(ctx, simNodes, simEdges);
      });
    sel.call(zoom as any);

    // Click detection
    canvas.addEventListener('click', (event) => {
      const [mx, my] = transform.invert([event.offsetX, event.offsetY]);
      const hit = simNodes.find(n => {
        const dx = (n.x ?? 0) - mx;
        const dy = (n.y ?? 0) - my;
        return dx * dx + dy * dy < 100;
      });
      selectedNode = hit ?? null;
      onSelectNode?.(selectedNode);
      draw(ctx, simNodes, simEdges);
    });

    // Hover detection
    canvas.addEventListener('mousemove', (event) => {
      const [mx, my] = transform.invert([event.offsetX, event.offsetY]);
      const hit = simNodes.find(n => {
        const dx = (n.x ?? 0) - mx;
        const dy = (n.y ?? 0) - my;
        return dx * dx + dy * dy < 100;
      });
      if (hit !== hoveredNode) {
        hoveredNode = hit ?? null;
        canvas.style.cursor = hit ? 'pointer' : 'default';
        draw(ctx, simNodes, simEdges);
      }
    });
  }

  function draw(ctx: CanvasRenderingContext2D, simNodes: GraphNode[], simEdges: GraphEdge[]) {
    ctx.save();
    ctx.clearRect(0, 0, width, height);
    ctx.translate(transform.x, transform.y);
    ctx.scale(transform.k, transform.k);

    // Edges
    ctx.strokeStyle = 'rgba(150, 150, 170, 0.15)';
    ctx.lineWidth = 0.5;
    for (const e of simEdges) {
      const src = e.source as GraphNode;
      const tgt = e.target as GraphNode;
      if (src.x == null || tgt.x == null) continue;
      ctx.beginPath();
      ctx.moveTo(src.x, src.y!);
      ctx.lineTo(tgt.x, tgt.y!);
      ctx.stroke();
    }

    // Nodes
    for (const n of simNodes) {
      if (n.x == null) continue;
      const r = nodeRadius(n);
      const isSelected = selectedNode?.id === n.id;
      const isHovered = hoveredNode?.id === n.id;

      ctx.beginPath();
      ctx.arc(n.x, n.y!, r, 0, Math.PI * 2);
      ctx.fillStyle = nodeColor(n);
      if (isSelected) {
        ctx.fillStyle = '#2d5bff';
        ctx.shadowColor = '#2d5bff';
        ctx.shadowBlur = 8;
      } else if (isHovered) {
        ctx.shadowColor = nodeColor(n);
        ctx.shadowBlur = 4;
      }
      ctx.fill();
      ctx.shadowBlur = 0;

      // Label: only show for selected/hovered, or for small graphs with large nodes
      if (isSelected || isHovered || (!isLargeGraph && r >= 6)) {
        ctx.fillStyle = isSelected ? '#2d5bff' : 'rgba(100, 100, 120, 0.8)';
        ctx.font = `${isSelected || isHovered ? 'bold ' : ''}${Math.max(8, 10 / transform.k)}px -apple-system, sans-serif`;
        ctx.textAlign = 'center';
        ctx.fillText(n.name, n.x, n.y! + r + 10);
      }
    }

    ctx.restore();
  }

  function handleResize() {
    if (!container) return;
    width = container.clientWidth;
    height = container.clientHeight;
    if (canvas) {
      canvas.width = width;
      canvas.height = height;
    }
  }

  $effect(() => {
    if (nodes.length > 0 && canvas) {
      simulation?.stop();
      setupSimulation();
    }
  });

  onMount(() => {
    handleResize();
    window.addEventListener('resize', handleResize);
  });

  onDestroy(() => {
    simulation?.stop();
    window.removeEventListener('resize', handleResize);
  });
</script>

<div bind:this={container} class="relative w-full h-full min-h-60 bg-surface-z1 rounded-lg overflow-hidden">
  <canvas bind:this={canvas} {width} {height} class="w-full h-full"></canvas>

  <!-- Legend -->
  <div class="absolute bottom-2 left-2 flex flex-wrap gap-x-3 gap-y-0.5 text-[9px] text-surface-z5 pointer-events-none">
    <span><span class="inline-block w-2 h-2 rounded-full mr-0.5" style="background:#6366f1"></span> functions</span>
    <span><span class="inline-block w-2 h-2 rounded-full mr-0.5" style="background:#f59e0b"></span> types</span>
    <span><span class="inline-block w-2 h-2 rounded-full mr-0.5" style="background:#10b981"></span> files</span>
    <span><span class="inline-block w-2 h-2 rounded-full mr-0.5" style="background:#06b6d4"></span> docs</span>
    <span><span class="inline-block w-2 h-2 rounded-full mr-0.5" style="background:#ec4899"></span> components</span>
  </div>

  {#if hoveredNode && !selectedNode}
    <div class="absolute top-2 left-2 rounded-md bg-surface-z2 border border-surface-z3 px-2.5 py-1.5 text-xs shadow-sm pointer-events-none">
      <span class="font-medium text-surface-z8">{hoveredNode.name}</span>
      <span class="text-surface-z4 ml-1">{hoveredNode.kind}</span>
      <span class="text-surface-z3 ml-1">{hoveredNode.file}:{hoveredNode.line}</span>
    </div>
  {/if}

  {#if selectedNode}
    <div class="absolute top-2 right-2 w-56 rounded-lg bg-surface-z2 border border-surface-z3 px-3 py-2.5 text-xs shadow-md">
      <div class="flex items-center justify-between mb-1.5">
        <span class="font-semibold text-surface-z8">{selectedNode.name}</span>
        <button onclick={() => { selectedNode = null; onSelectNode?.(null); }} class="text-surface-z4 hover:text-surface-z6">x</button>
      </div>
      <div class="space-y-1 text-surface-z5">
        <div>Kind: <span class="text-surface-z7">{selectedNode.kind}</span></div>
        <div>File: <span class="text-surface-z7 break-all">{selectedNode.file}:{selectedNode.line}</span></div>
        {#if selectedNode.complexity && selectedNode.complexity > 1}
          <div>Complexity: <span class="text-surface-z7">{selectedNode.complexity}</span></div>
        {/if}
      </div>
    </div>
  {/if}

  {#if nodes.length === 0}
    <div class="absolute inset-0 flex items-center justify-center text-xs text-surface-z4">
      No graph data. Index this repo first.
    </div>
  {/if}
</div>
