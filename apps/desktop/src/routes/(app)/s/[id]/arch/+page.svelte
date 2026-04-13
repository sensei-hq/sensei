<script lang="ts">
  import { page } from '$app/stores';
  import { onMount } from 'svelte';
  import { getSolutionById } from '$lib/solutions.svelte.js';
  import { senseiApi } from '$lib/api.js';
  import type { GraphData } from '$lib/types.js';
  import GraphCanvas from '$lib/GraphCanvas.svelte';

  let solution = $derived(getSolutionById($page.params.id as string));
  let port = $state(parseInt(localStorage.getItem('sensei:port') ?? '7744', 10));

  type ViewTab = 'graph' | 'structural' | 'deployment';
  let activeView = $state<ViewTab>('graph');

  // Raw node/edge data for D3
  let graphNodes = $state<Array<{ id: string; name: string; kind: string; file: string; line: number; complexity?: number }>>([]);
  let graphEdges = $state<Array<{ source: string; target: string; type: string }>>([]);
  let selectedGraphNode = $state<any>(null);

  // Aggregated graph data across all repos in the solution
  let repoGraphs = $state<Map<string, GraphData>>(new Map());
  let loading = $state(true);

  // Aggregated stats
  let totalSymbols = $derived([...repoGraphs.values()].reduce((sum, g) => sum + g.summary.totalSymbols, 0));
  let totalEdges = $derived([...repoGraphs.values()].reduce((sum, g) => sum + g.summary.totalEdges, 0));
  let totalCommunities = $derived([...repoGraphs.values()].reduce((sum, g) => sum + g.summary.communities, 0));

  let allGodNodes = $derived(
    [...repoGraphs.entries()].flatMap(([repoId, g]) =>
      g.godNodes.map(n => ({ ...n, repoLabel: solution?.repos.find(r => r.repoId === repoId)?.label ?? repoId }))
    ).sort((a, b) => b.degree - a.degree)
  );

  let allCommunities = $derived(
    [...repoGraphs.entries()].flatMap(([repoId, g]) =>
      g.communities.map(c => ({ ...c, repoLabel: solution?.repos.find(r => r.repoId === repoId)?.label ?? repoId }))
    ).sort((a, b) => b.symbolCount - a.symbolCount)
  );

  let allRationale = $derived(
    [...repoGraphs.values()].flatMap(g => g.rationale)
  );

  const TAG_CLS: Record<string, string> = {
    WHY: 'bg-info-z2 text-info-z6',
    DECISION: 'bg-primary-z2 text-primary-z6',
    HACK: 'bg-warning-z2 text-warning-z6',
    NOTE: 'bg-surface-z3 text-surface-z5',
  };

  async function loadGraphs() {
    if (!solution) return;
    loading = true;
    const api = senseiApi(port);
    const results = new Map<string, GraphData>();
    await Promise.all(solution.repos.map(async (repo) => {
      const data = await api.getGraph(repo.repoId, repo.path);
      if (data.summary.totalSymbols > 0) {
        results.set(repo.repoId, data);
      }
    }));
    repoGraphs = results;

    // Also load raw node/edge data for D3
    const allNodes: typeof graphNodes = [];
    const allEdges: typeof graphEdges = [];
    await Promise.all(solution.repos.map(async (repo) => {
      try {
        const res = await fetch(`http://127.0.0.1:${port}/api/graph/nodes?repoId=${encodeURIComponent(repo.repoId)}`);
        if (res.ok) {
          const data = await res.json() as { nodes: typeof graphNodes; edges: typeof graphEdges };
          allNodes.push(...data.nodes);
          allEdges.push(...data.edges);
        }
      } catch { /* ignore */ }
    }));
    graphNodes = allNodes;
    graphEdges = allEdges;

    loading = false;
  }

  onMount(() => { loadGraphs(); });
</script>

{#if solution}
  <div class="flex-1 overflow-y-auto px-6 py-5 space-y-6">

    <!-- View toggle -->
    <div class="flex items-center gap-2">
      {#each [['graph', 'Graph'], ['structural', 'Structural'], ['deployment', 'Deployment']] as [id, label]}
        <button
          onclick={() => activeView = id as ViewTab}
          class="rounded-md px-3 py-1.5 text-xs font-medium transition-colors
                 {activeView === id ? 'bg-primary-z2 text-primary-z7' : 'text-surface-z4 hover:text-surface-z6 hover:bg-surface-z3/40'}"
        >
          {label}
        </button>
      {/each}
    </div>

    {#if activeView === 'graph'}
      <!-- ═══ FORCE-DIRECTED GRAPH ═══ -->
      {#if loading}
        <div class="text-center py-12">
          <p class="text-sm text-surface-z4">Loading graph…</p>
        </div>
      {:else if graphNodes.length === 0}
        <div class="text-center py-12">
          <p class="text-sm text-surface-z4">No graph data. Index repos first.</p>
        </div>
      {:else}
        <div class="h-96 rounded-lg border border-surface-z0/50">
          <GraphCanvas
            nodes={graphNodes}
            edges={graphEdges}
            onSelectNode={(n) => { selectedGraphNode = n; }}
          />
        </div>
        <div class="flex gap-3 text-xs text-surface-z4">
          <span>{graphNodes.length} nodes</span>
          <span>{graphEdges.length} edges</span>
          {#if selectedGraphNode}
            <span class="text-primary-z6">Selected: {selectedGraphNode.name} ({selectedGraphNode.kind})</span>
          {/if}
        </div>
      {/if}

    {:else if activeView === 'structural'}
      <!-- ═══ STRUCTURAL VIEW ═══ -->

      {#if loading}
        <div class="text-center py-12">
          <p class="text-sm text-surface-z4">Loading graph data across {solution.repos.length} repos…</p>
        </div>
      {:else if repoGraphs.size === 0}
        <div class="text-center py-12">
          <p class="text-sm text-surface-z4">No indexed repos yet.</p>
          <p class="text-xs text-surface-z3 mt-1">Index repos first to see the structural view.</p>
        </div>
      {:else}

        <!-- Summary -->
        <div class="grid grid-cols-4 gap-4">
          <div class="rounded-lg bg-surface-z2 p-3">
            <p class="text-[10px] text-surface-z4 uppercase tracking-wide">Symbols</p>
            <p class="mt-1 text-xl font-semibold text-surface-z8">{totalSymbols.toLocaleString()}</p>
          </div>
          <div class="rounded-lg bg-surface-z2 p-3">
            <p class="text-[10px] text-surface-z4 uppercase tracking-wide">Edges</p>
            <p class="mt-1 text-xl font-semibold text-surface-z8">{totalEdges.toLocaleString()}</p>
          </div>
          <div class="rounded-lg bg-surface-z2 p-3">
            <p class="text-[10px] text-surface-z4 uppercase tracking-wide">Communities</p>
            <p class="mt-1 text-xl font-semibold text-surface-z8">{totalCommunities}</p>
          </div>
          <div class="rounded-lg bg-surface-z2 p-3">
            <p class="text-[10px] text-surface-z4 uppercase tracking-wide">God Nodes</p>
            <p class="mt-1 text-xl font-semibold {allGodNodes.length > 0 ? 'text-warning-z6' : 'text-success-z6'}">{allGodNodes.length}</p>
          </div>
        </div>

        <!-- God Nodes (pain points) -->
        {#if allGodNodes.length > 0}
          <div>
            <h3 class="text-xs font-semibold text-surface-z5 uppercase tracking-wide mb-2">
              God Nodes
              <span class="font-normal normal-case text-surface-z4 ml-1">— high coupling, wide blast radius</span>
            </h3>
            <div class="rounded-lg border border-surface-z0/50 divide-y divide-surface-z0/30">
              {#each allGodNodes.slice(0, 15) as node}
                <div class="flex items-center gap-3 px-3 py-2 text-xs">
                  <span class="h-2.5 w-2.5 rounded-full shrink-0
                    {node.degree >= 20 ? 'bg-error-z5' : node.degree >= 10 ? 'bg-warning-z5' : 'bg-info-z5'}"></span>
                  <span class="font-mono text-surface-z7 font-medium truncate flex-1">{node.name}</span>
                  <span class="text-surface-z4 shrink-0">degree {node.degree}</span>
                  <span class="rounded bg-surface-z3 px-1.5 py-0.5 text-[10px] text-surface-z5 shrink-0">{node.repoLabel}</span>
                  <span class="text-surface-z3 truncate max-w-40">{node.file}</span>
                </div>
              {/each}
            </div>
          </div>
        {/if}

        <!-- Communities -->
        {#if allCommunities.length > 0}
          <div>
            <h3 class="text-xs font-semibold text-surface-z5 uppercase tracking-wide mb-2">
              Code Communities
              <span class="font-normal normal-case text-surface-z4 ml-1">— clusters of related symbols</span>
            </h3>
            <div class="grid grid-cols-2 xl:grid-cols-3 gap-2">
              {#each allCommunities.slice(0, 12) as community}
                <div class="rounded-lg bg-surface-z2 px-3 py-2">
                  <div class="flex items-center gap-2">
                    <span class="rounded px-1.5 py-0.5 text-[10px] font-medium {community.color}">{community.symbolCount}</span>
                    <span class="text-xs text-surface-z6 truncate flex-1">{community.label}</span>
                  </div>
                  <p class="text-[10px] text-surface-z3 mt-1">{community.repoLabel}</p>
                  {#if community.godNodes.length > 0}
                    <p class="text-[10px] text-warning-z5 mt-0.5">
                      Hot: {community.godNodes.slice(0, 3).join(', ')}
                    </p>
                  {/if}
                </div>
              {/each}
            </div>
          </div>
        {/if}

        <!-- Rationale -->
        {#if allRationale.length > 0}
          <div>
            <h3 class="text-xs font-semibold text-surface-z5 uppercase tracking-wide mb-2">
              Rationale Comments
              <span class="font-normal normal-case text-surface-z4 ml-1">— WHY/DECISION/HACK annotations</span>
            </h3>
            <div class="space-y-1">
              {#each allRationale.slice(0, 20) as r}
                <div class="flex items-start gap-2 rounded-lg bg-surface-z2/50 px-3 py-2 text-xs">
                  <span class="rounded px-1.5 py-0.5 text-[10px] font-medium shrink-0 {TAG_CLS[r.tag] ?? TAG_CLS.NOTE}">{r.tag}</span>
                  <span class="text-surface-z6 flex-1">{r.text}</span>
                  <span class="text-surface-z3 shrink-0 truncate max-w-32">{r.file}</span>
                </div>
              {/each}
            </div>
          </div>
        {/if}

      {/if}

    {:else if activeView === 'deployment'}
      <!-- ═══ DEPLOYMENT VIEW ═══ -->
      <div class="text-center py-12">
        <span class="text-4xl i-solar-cloud-bold-duotone text-surface-z3"></span>
        <p class="text-sm text-surface-z4 mt-3">Deployment view coming in Phase 3</p>
        <p class="text-xs text-surface-z3 mt-1">Will auto-detect from Dockerfiles, k8s manifests, and compose files.</p>
      </div>
    {/if}

  </div>
{/if}
