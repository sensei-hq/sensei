<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { Tree } from '@rokkit/ui';
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

  // ── Tree filter ─────────────────────────────────────────────────────
  let treeFilterIds = $state<Set<string> | null>(null);
  let treeSelectedValue = $state<string | null>(null);
  let showTree = $state(true);

  const CONTAINER_EDGES = new Set(['CONTAINS_REPO', 'CONTAINS_GROUP', 'CONTAINS_PKG', 'CONTAINS_MOD']);
  const TREE_KINDS = new Set(['solution', 'repo', 'code-group', 'doc-group', 'package', 'module']);
  const KIND_ICONS: Record<string, string> = {
    'solution': 'i-lucide:layers', 'repo': 'i-lucide:git-branch',
    'code-group': 'i-lucide:code', 'doc-group': 'i-lucide:book-open',
    'package': 'i-lucide:package', 'module': 'i-lucide:puzzle',
  };

  // Build tree items for Rokkit Tree
  let treeItems = $derived.by(() => {
    const childMap = new Map<string, string[]>();
    for (const e of edges) {
      const t = typeof e.type === 'string' ? e.type : '';
      if (CONTAINER_EDGES.has(t)) {
        const src = typeof e.source === 'string' ? e.source : e.source.id;
        const tgt = typeof e.target === 'string' ? e.target : e.target.id;
        if (!childMap.has(src)) childMap.set(src, []);
        childMap.get(src)!.push(tgt);
      }
    }
    const nodeMap = new Map(nodes.map(n => [n.id, n]));

    // Count descendants via all containment edges
    const allContains = new Map<string, Set<string>>();
    for (const e of edges) {
      const t = typeof e.type === 'string' ? e.type : '';
      if (t.startsWith('CONTAINS_') || t === 'EXPORTS_FN' || t === 'EXPORTS_TYPE' || t === 'HAS_METHOD') {
        const src = typeof e.source === 'string' ? e.source : e.source.id;
        const tgt = typeof e.target === 'string' ? e.target : e.target.id;
        if (!allContains.has(src)) allContains.set(src, new Set());
        allContains.get(src)!.add(tgt);
      }
    }
    function countDesc(id: string, visited = new Set<string>()): number {
      if (visited.has(id)) return 0; visited.add(id);
      const ch = allContains.get(id); if (!ch) return 0;
      let c = 0;
      for (const cid of ch) { const cn = nodeMap.get(cid); if (cn && !TREE_KINDS.has(cn.kind)) c++; c += countDesc(cid, visited); }
      return c;
    }

    function build(id: string, depth: number): any {
      const node = nodeMap.get(id);
      if (!node || !TREE_KINDS.has(node.kind) || depth > 3) return null;
      const childIds = childMap.get(id) ?? [];
      const children: any[] = [];
      for (const cid of childIds) { const c = build(cid, depth + 1); if (c) children.push(c); }
      children.sort((a: any, b: any) => a.text.localeCompare(b.text));
      const cnt = countDesc(id);
      return { value: id, label: node.name, icon: KIND_ICONS[node.kind] ?? '', badge: cnt > 0 ? String(cnt) : '', children, expanded: depth < 2 };
    }

    const hasParent = new Set<string>();
    for (const e of edges) { const t = typeof e.type === 'string' ? e.type : ''; if (CONTAINER_EDGES.has(t)) { const tgt = typeof e.target === 'string' ? e.target : e.target.id; hasParent.add(tgt); } }
    const roots: any[] = [];
    for (const n of nodes) { if (TREE_KINDS.has(n.kind) && !hasParent.has(n.id)) { const t = build(n.id, 0); if (t) roots.push(t); } }
    return roots;
  });

  function collectSubtreeIds(nodeId: string): Set<string> {
    const ids = new Set<string>();
    const queue = [nodeId];
    while (queue.length > 0) {
      const cur = queue.pop()!;
      if (ids.has(cur)) continue; ids.add(cur);
      for (const e of edges) {
        const src = typeof e.source === 'string' ? e.source : e.source.id;
        const tgt = typeof e.target === 'string' ? e.target : e.target.id;
        const t = typeof e.type === 'string' ? e.type : '';
        if (src === cur && (t.startsWith('CONTAINS_') || t === 'EXPORTS_FN' || t === 'EXPORTS_TYPE' || t === 'HAS_METHOD')) {
          queue.push(tgt);
        }
      }
    }
    return ids;
  }

  function handleTreeSelect(value: unknown) {
    const id = value as string;
    if (treeSelectedValue === id) { treeSelectedValue = null; treeFilterIds = null; }
    else { treeSelectedValue = id; treeFilterIds = collectSubtreeIds(id); }
  }

  // Filtered nodes — simple: if tree filter active, use it; otherwise show all
  let filteredNodes = $derived.by(() => {
    if (!treeFilterIds) return nodes;
    return nodes.filter(n => treeFilterIds!.has(n.id));
  });

  let filteredEdges = $derived.by(() => {
    const ids = new Set(filteredNodes.map(n => n.id));
    return edges.filter(e => {
      const src = typeof e.source === 'string' ? e.source : e.source.id;
      const tgt = typeof e.target === 'string' ? e.target : e.target.id;
      return ids.has(src) && ids.has(tgt);
    });
  });

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
    'repo': '#ef4444', 'solution': '#dc2626',
    'code-group': '#3b82f6', 'doc-group': '#06b6d4',
  };

  function nodeColor(node: GraphNode): string {
    return KIND_COLORS[node.kind] ?? '#94a3b8';
  }

  function nodeRadius(node: GraphNode): number {
    if (node.kind === 'solution') return isLargeGraph ? 10 : 20;
    if (node.kind === 'repo' || node.kind === 'code-group' || node.kind === 'doc-group') return isLargeGraph ? 8 : 16;
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
        || n.kind === 'solution' || n.kind === 'repo'
        || n.kind === 'package' || n.kind === 'module'
        || n.kind === 'code-group' || n.kind === 'doc-group'
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
    treeSelectedValue = null;
    treeFilterIds = null;
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

<div bind:this={container} class="relative w-full h-full min-h-60 bg-surface-z1 rounded-lg overflow-hidden flex">
  <!-- Canvas (main area) -->
  <div class="flex-1 relative min-h-0 flex flex-col">
    <!-- Top bar -->
    <div class="flex items-center gap-2 px-2 py-1 border-b border-surface-z0/30 bg-surface-z2/50 shrink-0 text-[10px]">
      <button onclick={() => showTree = !showTree} class="text-surface-z5 hover:text-surface-z7" title="Toggle tree">{showTree ? '◂' : '▸'} Tree</button>
      {#if treeFilterIds}
        <button onclick={clearFilters} class="text-primary-z6 hover:text-primary-z7 font-medium">Clear filter</button>
      {/if}
      <button onclick={() => zoomToFit()} class="text-surface-z5 hover:text-surface-z7 font-medium" title="Zoom to fit">Fit</button>
      <span class="text-surface-z4 ml-auto">{filteredNodes.length} nodes · {filteredEdges.length} edges</span>
    </div>
    <div class="flex-1 relative min-h-0">
      <canvas bind:this={canvas} {width} {height} class="w-full h-full"></canvas>
    </div>
    <!-- Legend -->
    <div class="absolute bottom-2 left-2 flex flex-wrap gap-x-3 gap-y-0.5 text-[9px] text-surface-z5 pointer-events-none">
      <span><span class="inline-block w-2 h-2 rounded-full mr-0.5" style="background:#8b5cf6"></span> packages</span>
      <span><span class="inline-block w-2 h-2 rounded-full mr-0.5" style="background:#14b8a6"></span> modules</span>
      <span><span class="inline-block w-2 h-2 rounded-full mr-0.5" style="background:#6366f1"></span> functions</span>
      <span><span class="inline-block w-2 h-2 rounded-full mr-0.5" style="background:#f59e0b"></span> types</span>
      <span><span class="inline-block w-2 h-2 rounded-full mr-0.5" style="background:#f97316"></span> extensions</span>
      <span><span class="inline-block w-2 h-2 rounded-full mr-0.5" style="background:#06b6d4"></span> docs</span>
    </div>
  </div>

  <!-- Tree panel (right side) -->
  {#if showTree && treeItems.length > 0}
    <div class="w-56 shrink-0 border-l border-surface-z0/30 bg-surface-z2/30 overflow-y-auto">
      <Tree
        items={treeItems}
        value={treeSelectedValue}
        size="sm"
        lineStyle="dashed"
        icons={{ opened: 'i-lucide:chevron-down', closed: 'i-lucide:chevron-right' }}
        onselect={handleTreeSelect}
      />
    </div>
  {/if}

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
