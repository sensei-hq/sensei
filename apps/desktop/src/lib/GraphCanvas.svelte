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

  // Keep references for zoom-to-fit
  let currentSimNodes: GraphNode[] = [];
  let currentCtx: CanvasRenderingContext2D | null = null;
  let currentSimEdges: GraphEdge[] = [];
  let zoomBehavior: d3Zoom.ZoomBehavior<HTMLCanvasElement, unknown> | null = null;

  // ── Filters ────────────────────────────────────────────────────────
  let filterPackage = $state<string>('all');
  let filterModule = $state<string>('all');
  let filterKind = $state<string>('all');

  let packages = $derived(nodes.filter(n => n.kind === 'package').map(n => n.name).sort());
  let kindOptions = $derived([...new Set(nodes.map(n => n.kind))].sort());

  // Modules: filtered by selected package
  let modules = $derived.by(() => {
    const allMods = nodes.filter(n => n.kind === 'module');
    if (filterPackage === 'all') return allMods.map(n => n.name).sort();
    const pkgNode = nodes.find(n => n.kind === 'package' && n.name === filterPackage);
    if (!pkgNode) return allMods.map(n => n.name).sort();
    const modIds = pkgContainsMods.get(pkgNode.id) ?? new Set();
    return allMods.filter(n => modIds.has(n.id)).map(n => n.name).sort();
  });

  $effect(() => {
    if (filterModule !== 'all' && !modules.includes(filterModule)) {
      filterModule = 'all';
    }
  });

  // Build containment maps
  let modContains = $derived.by(() => {
    const map = new Map<string, Set<string>>();
    for (const e of edges) {
      const t = typeof e.type === 'string' ? e.type : '';
      if (t === 'CONTAINS_FN' || t === 'CONTAINS_FILE') {
        const src = typeof e.source === 'string' ? e.source : e.source.id;
        const tgt = typeof e.target === 'string' ? e.target : e.target.id;
        if (!map.has(src)) map.set(src, new Set());
        map.get(src)!.add(tgt);
      }
    }
    return map;
  });

  let pkgContainsMods = $derived.by(() => {
    const map = new Map<string, Set<string>>();
    for (const e of edges) {
      const t = typeof e.type === 'string' ? e.type : '';
      if (t === 'CONTAINS_MOD') {
        const src = typeof e.source === 'string' ? e.source : e.source.id;
        const tgt = typeof e.target === 'string' ? e.target : e.target.id;
        if (!map.has(src)) map.set(src, new Set());
        map.get(src)!.add(tgt);
      }
    }
    return map;
  });

  // Filtered nodes
  let filteredNodes = $derived.by(() => {
    let result = nodes;
    if (filterKind !== 'all') {
      result = result.filter(n => n.kind === filterKind);
    }
    if (filterPackage !== 'all') {
      const pkgNode = nodes.find(n => n.kind === 'package' && n.name === filterPackage);
      if (pkgNode) {
        const modIds = pkgContainsMods.get(pkgNode.id) ?? new Set();
        const nodeIds = new Set<string>();
        nodeIds.add(pkgNode.id);
        for (const modId of modIds) {
          nodeIds.add(modId);
          const contained = modContains.get(modId);
          if (contained) for (const id of contained) nodeIds.add(id);
        }
        result = result.filter(n => nodeIds.has(n.id));
      }
    }
    if (filterModule !== 'all') {
      const modNode = nodes.find(n => n.kind === 'module' && n.name === filterModule);
      if (modNode) {
        const nodeIds = new Set<string>();
        nodeIds.add(modNode.id);
        const contained = modContains.get(modNode.id);
        if (contained) for (const id of contained) nodeIds.add(id);
        for (const e of edges) {
          const t = typeof e.type === 'string' ? e.type : '';
          if (t === 'HAS_METHOD' || t === 'EXPORTS_TYPE' || t === 'EXPORTS_FN') {
            const src = typeof e.source === 'string' ? e.source : e.source.id;
            const tgt = typeof e.target === 'string' ? e.target : e.target.id;
            if (nodeIds.has(tgt)) nodeIds.add(src);
            if (nodeIds.has(src)) nodeIds.add(tgt);
          }
        }
        result = result.filter(n => nodeIds.has(n.id));
      }
    }
    return result;
  });

  let filteredEdges = $derived.by(() => {
    const ids = new Set(filteredNodes.map(n => n.id));
    return edges.filter(e => {
      const src = typeof e.source === 'string' ? e.source : e.source.id;
      const tgt = typeof e.target === 'string' ? e.target : e.target.id;
      return ids.has(src) && ids.has(tgt);
    });
  });

  let activeFilters = $derived(
    (filterPackage !== 'all' ? 1 : 0) +
    (filterModule !== 'all' ? 1 : 0) +
    (filterKind !== 'all' ? 1 : 0)
  );

  // ── Colors ─────────────────────────────────────────────────────────
  const KIND_COLORS: Record<string, string> = {
    'function': '#6366f1', 'method': '#6366f1',
    'class': '#f59e0b', 'struct': '#f59e0b', 'interface': '#f59e0b',
    'type': '#f59e0b', 'enum': '#f59e0b',
    'file': '#10b981',
    'doc': '#06b6d4', 'extension': '#f97316', // orange for extensions
    'component': '#ec4899', 'hook': '#ec4899',
    'const': '#94a3b8',
    'package': '#8b5cf6', 'module': '#14b8a6',
    'repo': '#ef4444', 'code-group': '#3b82f6', 'doc-group': '#06b6d4',
  };

  function nodeColor(node: GraphNode): string {
    return KIND_COLORS[node.kind] ?? '#94a3b8';
  }

  function nodeRadius(node: GraphNode): number {
    if (node.kind === 'package') return isLargeGraph ? 6 : 14;
    if (node.kind === 'module') return isLargeGraph ? 4 : 10;
    if (node.kind === 'class' || node.kind === 'struct') return isLargeGraph ? 3 : 8;
    if (isLargeGraph) return 2;
    return Math.max(3, Math.min(8, 3 + (node.complexity ?? 1) * 0.3));
  }

  let isLargeGraph = $derived(filteredNodes.length > 200);

  // ── Zoom to fit ────────────────────────────────────────────────────

  function zoomToFit(padding = 40) {
    if (!canvas || !zoomBehavior || currentSimNodes.length === 0) return;

    const positioned = currentSimNodes.filter(n => n.x != null && n.y != null);
    if (positioned.length === 0) return;

    let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
    for (const n of positioned) {
      const r = nodeRadius(n);
      if (n.x! - r < minX) minX = n.x! - r;
      if (n.y! - r < minY) minY = n.y! - r;
      if (n.x! + r > maxX) maxX = n.x! + r;
      if (n.y! + r > maxY) maxY = n.y! + r;
    }

    const bw = maxX - minX;
    const bh = maxY - minY;
    if (bw <= 0 || bh <= 0) return;

    const scale = Math.min(
      (width - padding * 2) / bw,
      (height - padding * 2) / bh,
      3 // cap zoom-in for small graphs
    );
    const cx = (minX + maxX) / 2;
    const cy = (minY + maxY) / 2;

    const newTransform = d3Zoom.zoomIdentity
      .translate(width / 2, height / 2)
      .scale(scale)
      .translate(-cx, -cy);

    const sel = d3Selection.select(canvas);
    sel.transition()
      .duration(500)
      .call(zoomBehavior.transform as any, newTransform);
  }

  // ── Simulation ─────────────────────────────────────────────────────

  function setupSimulation() {
    if (!canvas || filteredNodes.length === 0) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const simNodes = filteredNodes.map(n => ({ ...n }));
    const nodeIds = new Set(simNodes.map(n => n.id));
    const simEdges = filteredEdges
      .map(e => ({ ...e }))
      .filter(e => {
        const srcId = typeof e.source === 'string' ? e.source : e.source.id;
        const tgtId = typeof e.target === 'string' ? e.target : e.target.id;
        return nodeIds.has(srcId) && nodeIds.has(tgtId);
      });

    // Store refs for zoomToFit
    currentSimNodes = simNodes;
    currentSimEdges = simEdges;
    currentCtx = ctx;

    const large = simNodes.length > 200;
    const charge = large ? -20 : -80;
    const linkDist = large ? 30 : 60;
    const linkStr = large ? 0.1 : 0.3;

    const linkForce = d3Force.forceLink<GraphNode, GraphEdge>(simEdges)
      .id((d: GraphNode) => d.id)
      .distance((d: any) => {
        const t = d.type ?? '';
        if (t === 'CONTAINS_PKG' || t === 'CONTAINS_MOD') return linkDist * 0.5;
        if (t === 'CONTAINS_FN' || t === 'HAS_METHOD') return linkDist * 0.7;
        return linkDist;
      })
      .strength((d: any) => {
        const t = d.type ?? '';
        if (t.startsWith('CONTAINS') || t === 'HAS_METHOD') return linkStr * 2;
        return linkStr;
      });

    simulation = d3Force.forceSimulation(simNodes)
      .force('link', linkForce)
      .force('charge', d3Force.forceManyBody().strength(charge))
      .force('center', d3Force.forceCenter(width / 2, height / 2))
      .force('collision', d3Force.forceCollide().radius((d: any) => nodeRadius(d) + 1))
      .on('tick', () => draw(ctx, simNodes, simEdges))
      .on('end', () => zoomToFit());

    // Zoom
    const sel = d3Selection.select(canvas);
    zoomBehavior = d3Zoom.zoom<HTMLCanvasElement, unknown>()
      .scaleExtent([0.05, 8])
      .on('zoom', (event: any) => {
        transform = event.transform;
        draw(ctx, simNodes, simEdges);
      });
    sel.call(zoomBehavior as any);

    // Click
    canvas.addEventListener('click', (event) => {
      const [mx, my] = transform.invert([event.offsetX, event.offsetY]);
      const hit = simNodes.find(n => {
        const dx = (n.x ?? 0) - mx;
        const dy = (n.y ?? 0) - my;
        const r = nodeRadius(n);
        return dx * dx + dy * dy < r * r + 25;
      });
      selectedNode = hit ?? null;
      onSelectNode?.(selectedNode);
      draw(ctx, simNodes, simEdges);
    });

    // Hover
    canvas.addEventListener('mousemove', (event) => {
      const [mx, my] = transform.invert([event.offsetX, event.offsetY]);
      const hit = simNodes.find(n => {
        const dx = (n.x ?? 0) - mx;
        const dy = (n.y ?? 0) - my;
        const r = nodeRadius(n);
        return dx * dx + dy * dy < r * r + 25;
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
    for (const e of simEdges) {
      const src = e.source as GraphNode;
      const tgt = e.target as GraphNode;
      if (src.x == null || tgt.x == null) continue;
      const t = typeof e.type === 'string' ? e.type : '';
      ctx.strokeStyle = t.startsWith('CONTAINS') || t === 'HAS_METHOD'
        ? 'rgba(139, 92, 246, 0.2)'
        : t === 'CALLS' ? 'rgba(99, 102, 241, 0.25)'
        : 'rgba(150, 150, 170, 0.12)';
      ctx.lineWidth = t.startsWith('CONTAINS') ? 0.8 : 0.4;
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

      const showLabel = isSelected || isHovered
        || n.kind === 'package' || n.kind === 'module'
        || (!isLargeGraph && r >= 6);
      if (showLabel) {
        ctx.fillStyle = isSelected ? '#2d5bff' : 'rgba(100, 100, 120, 0.8)';
        const fontSize = n.kind === 'package' ? 11 : n.kind === 'module' ? 10 : Math.max(8, 10 / transform.k);
        ctx.font = `${isSelected || isHovered || n.kind === 'package' ? 'bold ' : ''}${fontSize}px -apple-system, sans-serif`;
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

  function clearFilters() {
    filterPackage = 'all';
    filterModule = 'all';
    filterKind = 'all';
  }

  $effect(() => {
    if (filteredNodes.length >= 0 && canvas) {
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

<div bind:this={container} class="relative w-full h-full min-h-60 bg-surface-z1 rounded-lg overflow-hidden flex flex-col">
  <!-- Filter bar -->
  {#if packages.length > 0 || modules.length > 0}
    <div class="flex items-center gap-2 px-2 py-1.5 border-b border-surface-z0/30 bg-surface-z2/50 shrink-0 text-[10px]">
      <span class="text-surface-z5 font-medium uppercase tracking-wide">Filter</span>
      {#if packages.length > 0}
        <select bind:value={filterPackage} class="bg-surface-z2 border border-surface-z0/40 rounded px-1.5 py-0.5 text-surface-z7 text-[10px]">
          <option value="all">All packages</option>
          {#each packages as p}
            <option value={p}>{p}</option>
          {/each}
        </select>
      {/if}
      {#if modules.length > 0}
        <select bind:value={filterModule} class="bg-surface-z2 border border-surface-z0/40 rounded px-1.5 py-0.5 text-surface-z7 text-[10px]">
          <option value="all">{filterPackage !== 'all' ? 'All modules in package' : 'All modules'}</option>
          {#each modules as m}
            <option value={m}>{m}</option>
          {/each}
        </select>
      {/if}
      <select bind:value={filterKind} class="bg-surface-z2 border border-surface-z0/40 rounded px-1.5 py-0.5 text-surface-z7 text-[10px]">
        <option value="all">All kinds</option>
        {#each kindOptions as k}
          <option value={k}>{k}</option>
        {/each}
      </select>
      {#if activeFilters > 0}
        <button onclick={clearFilters} class="text-primary-z6 hover:text-primary-z7 font-medium">Clear</button>
      {/if}
      <button onclick={() => zoomToFit()} class="text-surface-z5 hover:text-surface-z7 font-medium ml-1" title="Zoom to fit">Fit</button>
      <span class="text-surface-z4 ml-auto">{filteredNodes.length} nodes · {filteredEdges.length} edges</span>
    </div>
  {/if}

  <!-- Canvas -->
  <div class="flex-1 relative min-h-0">
    <canvas bind:this={canvas} {width} {height} class="w-full h-full"></canvas>
  </div>

  <!-- Legend -->
  <div class="absolute bottom-2 left-2 flex flex-wrap gap-x-3 gap-y-0.5 text-[9px] text-surface-z5 pointer-events-none">
    <span><span class="inline-block w-2 h-2 rounded-full mr-0.5" style="background:#8b5cf6"></span> packages</span>
    <span><span class="inline-block w-2 h-2 rounded-full mr-0.5" style="background:#14b8a6"></span> modules</span>
    <span><span class="inline-block w-2 h-2 rounded-full mr-0.5" style="background:#6366f1"></span> functions</span>
    <span><span class="inline-block w-2 h-2 rounded-full mr-0.5" style="background:#f59e0b"></span> types</span>
    <span><span class="inline-block w-2 h-2 rounded-full mr-0.5" style="background:#10b981"></span> files</span>
    <span><span class="inline-block w-2 h-2 rounded-full mr-0.5" style="background:#06b6d4"></span> docs</span>
  </div>

  {#if hoveredNode && !selectedNode}
    <div class="absolute bottom-8 right-2 rounded-md bg-surface-z2 border border-surface-z3 px-2.5 py-1.5 text-xs shadow-sm pointer-events-none z-10">
      <span class="font-medium text-surface-z8">{hoveredNode.name}</span>
      <span class="text-surface-z5 ml-1">{hoveredNode.kind}</span>
      {#if hoveredNode.file}
        <span class="text-surface-z4 ml-1">{hoveredNode.file.split('/').slice(-2).join('/')}</span>
      {/if}
    </div>
  {/if}

  {#if selectedNode}
    <div class="absolute top-2 right-2 w-56 rounded-lg bg-surface-z2 border border-surface-z3 px-3 py-2.5 text-xs shadow-md z-10">
      <div class="flex items-center justify-between mb-1.5">
        <span class="font-semibold text-surface-z8">{selectedNode.name}</span>
        <button onclick={() => { selectedNode = null; onSelectNode?.(null); }} class="text-surface-z4 hover:text-surface-z6">x</button>
      </div>
      <div class="space-y-1 text-surface-z5">
        <div>Kind: <span class="text-surface-z7">{selectedNode.kind}</span></div>
        {#if selectedNode.file}
          <div>File: <span class="text-surface-z7 break-all">{selectedNode.file.split('/').slice(-2).join('/')}</span></div>
        {/if}
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
