<script lang="ts">
  import { getPort } from '$lib/appstate.svelte.js';
  import { page } from '$app/stores';
  import { onMount } from 'svelte';
  import { getSolutionById } from '$lib/solutions.svelte.js';
  import { senseiApi } from '$lib/api.js';
  import type { GraphData, CommunityInfo, FunctionDetail, DocDrift } from '$lib/types.js';
  import GraphCanvas from '$lib/GraphCanvas.svelte';

  let solution = $derived(getSolutionById($page.params.id as string));
  let port = $derived(getPort());

  type ViewTab = 'graph' | 'structural' | 'deployment';
  let activeView = $state<ViewTab>('graph');

  // Raw node/edge data for D3
  let graphNodes = $state<Array<{ id: string; name: string; kind: string; file: string; line: number; complexity?: number }>>([]);
  let graphEdges = $state<Array<{ source: string; target: string; type: string }>>([]);
  let selectedGraphNode = $state<any>(null);

  // Aggregated graph data across all repos in the solution
  let repoGraphs = $state<Map<string, GraphData>>(new Map());
  let loading = $state(true);

  // Community and structural data from daemon
  let communities = $state<Array<CommunityInfo & { repoId: string }>>([]);
  let highComplexityFns = $state<Array<FunctionDetail & { repoId: string }>>([]);
  let docDrift = $state<DocDrift[]>([]);

  // Aggregated stats
  let totalSymbols = $derived(graphNodes.length);
  let totalEdges = $derived(graphEdges.length);
  let totalCommunities = $derived(communities.length);

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

    // Load per-repo graph data for structural analysis
    const results = new Map<string, GraphData>();
    await Promise.all(solution.repos.map(async (repo) => {
      const data = await api.getGraph(repo.repoId);
      if (data.summary.totalSymbols > 0) {
        results.set(repo.repoId, data);
      }
    }));
    repoGraphs = results;

    // Load merged graph for D3 visualization
    try {
      const sg = await api.getSolutionGraph(solution.id);
      if (sg?.graph) {
        graphNodes = sg.graph.nodes;
        graphEdges = sg.graph.edges;
      }
    } catch {
      // Fallback: per-repo node/edge fetch
      const allNodes: typeof graphNodes = [];
      const allEdges: typeof graphEdges = [];
      for (const repo of solution.repos) {
        const data = await api.getGraphNodes(repo.repoId);
        allNodes.push(...data.nodes);
        allEdges.push(...data.edges);
      }
      graphNodes = allNodes;
      graphEdges = allEdges;
    }

    // Load communities and high-complexity functions per repo
    const allCommunities: typeof communities = [];
    const allHighFns: typeof highComplexityFns = [];
    const allDrift: DocDrift[] = [];
    await Promise.all((solution?.repos ?? []).map(async (repo) => {
      const comms = await api.getCommunities(repo.repoId);
      allCommunities.push(...comms.map(c => ({ ...c, repoId: repo.repoId })));

      // Get high complexity functions (search all, sort by complexity)
      const fns = await api.searchFunctions(repo.repoId, '');
      const high = fns.filter(f => f.complexity > 10).sort((a, b) => b.complexity - a.complexity).slice(0, 10);
      allHighFns.push(...high.map(f => ({ ...f, repoId: repo.repoId })));

      const drift = await api.getDocDrift(repo.repoId);
      allDrift.push(...drift);
    }));
    communities = allCommunities.sort((a, b) => b.size - a.size);
    highComplexityFns = allHighFns.sort((a, b) => b.complexity - a.complexity).slice(0, 20);
    docDrift = allDrift;

    loading = false;
  }

  onMount(() => { loadGraphs(); });
</script>

{#if solution}
  <div class="h-full overflow-y-auto px-6 py-5 space-y-6">

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
        <div class="h-[calc(100vh-14rem)] min-h-96 rounded-lg border border-surface-z0/50">
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
      {:else if graphNodes.length === 0}
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
            <p class="text-[10px] text-surface-z4 uppercase tracking-wide">Complex Functions</p>
            <p class="mt-1 text-xl font-semibold {highComplexityFns.length > 0 ? 'text-warning-z6' : 'text-success-z6'}">{highComplexityFns.length}</p>
          </div>
        </div>

        <!-- High Complexity Functions (pain points) -->
        {#if highComplexityFns.length > 0}
          <div>
            <h3 class="text-xs font-semibold text-surface-z5 uppercase tracking-wide mb-2">
              High Complexity
              <span class="font-normal normal-case text-surface-z4 ml-1">— functions with cyclomatic complexity > 10</span>
            </h3>
            <div class="rounded-lg border border-surface-z0/50 divide-y divide-surface-z0/30">
              {#each highComplexityFns as fn}
                <div class="flex items-center gap-3 px-3 py-2 text-xs">
                  <span class="h-2.5 w-2.5 rounded-full shrink-0
                    {fn.complexity >= 30 ? 'bg-error-z5' : fn.complexity >= 15 ? 'bg-warning-z5' : 'bg-info-z5'}"></span>
                  <span class="font-mono text-surface-z7 font-medium truncate flex-1">{fn.name}</span>
                  <span class="text-surface-z4 shrink-0">cx:{fn.complexity}</span>
                  <span class="text-surface-z3 truncate max-w-48">{fn.file.split('/').slice(-2).join('/')}</span>
                </div>
              {/each}
            </div>
          </div>
        {/if}

        <!-- Communities -->
        {#if communities.length > 0}
          <div>
            <h3 class="text-xs font-semibold text-surface-z5 uppercase tracking-wide mb-2">
              Code Communities
              <span class="font-normal normal-case text-surface-z4 ml-1">— Leiden-detected clusters of related symbols</span>
            </h3>
            <div class="grid grid-cols-2 xl:grid-cols-3 gap-2">
              {#each communities.slice(0, 15) as community}
                <div class="rounded-lg bg-surface-z2 px-3 py-2">
                  <div class="flex items-center gap-2">
                    <span class="rounded px-1.5 py-0.5 text-[10px] font-medium bg-primary-z2 text-primary-z7">{community.size}</span>
                    <span class="text-[10px] text-surface-z4">#{community.id}</span>
                  </div>
                  <p class="text-xs text-surface-z6 mt-1 truncate">{community.sample_members.join(', ')}</p>
                  <p class="text-[10px] text-surface-z3 mt-0.5">{community.repoId}</p>
                </div>
              {/each}
            </div>
          </div>
        {/if}

        <!-- Doc Drift -->
        {#if docDrift.length > 0}
          <div>
            <h3 class="text-xs font-semibold text-surface-z5 uppercase tracking-wide mb-2">
              Doc Drift
              <span class="font-normal normal-case text-surface-z4 ml-1">— documentation that may be stale</span>
            </h3>
            <div class="space-y-1">
              {#each docDrift.slice(0, 10) as d}
                <div class="flex items-center gap-2 rounded-lg bg-warning-z1 px-3 py-2 text-xs">
                  <span class="text-warning-z6 shrink-0">{d.edge_type}</span>
                  <span class="text-surface-z6 flex-1 truncate">{d.doc_path.split('/').slice(-2).join('/')}</span>
                  <span class="text-surface-z4 shrink-0">→ {d.changed_target.split(':').slice(-1)[0]}</span>
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
