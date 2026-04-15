<script lang="ts">
  import { getPort } from '$lib/appstate.svelte.js';
  import { page } from '$app/stores';
  import { onMount } from 'svelte';
  import { senseiApi } from '$lib/api.js';
  import GraphCanvas from '$lib/GraphCanvas.svelte';
  import type { ProjectSummary, GraphNode, GraphEdge, CommunityInfo, FunctionDetail, DocDrift } from '$lib/types.js';

  let repoId = $derived($page.params.id as string);
  let port = $derived(getPort());

  let summary = $state<ProjectSummary | null>(null);
  let graphNodes = $state<GraphNode[]>([]);
  let graphEdges = $state<GraphEdge[]>([]);
  let communities = $state<CommunityInfo[]>([]);
  let highComplexityFns = $state<FunctionDetail[]>([]);
  let docDrift = $state<DocDrift[]>([]);
  let sessions = $state<Array<{ id: string; task: string; project: string; ftr?: number | null; startedAt: string; outcome?: string; cost?: number }>>([]);
  let loading = $state(true);

  function ftrClass(ftr: number | null | undefined): string {
    if (ftr == null) return 'text-surface-z4';
    if (ftr >= 0.8) return 'text-success-z6';
    if (ftr >= 0.5) return 'text-warning-z6';
    return 'text-error-z6';
  }

  async function load() {
    const api = senseiApi(port);
    // Detect communities first (needs POST), then fetch all data in parallel
    await api.detectCommunities(repoId);
    const [s, graph, comms, drift, sessionData] = await Promise.all([
      api.getProjectSummary(repoId),
      api.getGraphNodes(repoId),
      api.getCommunities(repoId),
      api.getDocDrift(repoId),
      api.getSessions(),
    ]);
    summary = s;
    graphNodes = graph.nodes;
    graphEdges = graph.edges;
    communities = comms;
    docDrift = drift;
    // Filter sessions to this project
    sessions = sessionData.sessions.filter(s => s.project === repoId);

    // High complexity functions
    const fns = await api.searchFunctions(repoId, '');
    highComplexityFns = fns.filter(f => f.complexity > 10).sort((a, b) => b.complexity - a.complexity).slice(0, 15);

    loading = false;
  }

  onMount(() => { load(); });
</script>

<div class="h-full overflow-y-auto px-6 py-5 space-y-6">

  {#if loading}
    <p class="text-sm text-surface-z4 py-8 text-center">Loading...</p>
  {:else if summary}

    <!-- Header -->
    <div class="flex items-start justify-between gap-4">
      <div>
        <h2 class="text-lg font-semibold text-surface-z8">{summary.name}</h2>
        <p class="text-xs text-surface-z3 font-mono">{summary.path}</p>
      </div>
      <div class="flex items-center gap-2">
        {#if summary.indexedAt}
          <span class="rounded px-2 py-0.5 text-[10px] bg-success-z2 text-success-z7">indexed</span>
        {:else}
          <span class="rounded px-2 py-0.5 text-[10px] bg-surface-z3 text-surface-z5">not indexed</span>
        {/if}
      </div>
    </div>

    <!-- Stats -->
    <div class="grid grid-cols-4 xl:grid-cols-7 gap-3">
      <div class="rounded-lg bg-surface-z2 p-3">
        <p class="text-[10px] text-surface-z6 uppercase tracking-wide font-medium">Packages</p>
        <p class="mt-1 text-xl font-semibold text-violet-500">{(summary.packages ?? 0).toLocaleString()}</p>
      </div>
      <div class="rounded-lg bg-surface-z2 p-3">
        <p class="text-[10px] text-surface-z6 uppercase tracking-wide font-medium">Modules</p>
        <p class="mt-1 text-xl font-semibold text-teal-500">{(summary.modules ?? 0).toLocaleString()}</p>
      </div>
      <div class="rounded-lg bg-surface-z2 p-3">
        <p class="text-[10px] text-surface-z6 uppercase tracking-wide font-medium">Functions</p>
        <p class="mt-1 text-xl font-semibold text-indigo-400">{summary.functions.toLocaleString()}</p>
      </div>
      <div class="rounded-lg bg-surface-z2 p-3">
        <p class="text-[10px] text-surface-z6 uppercase tracking-wide font-medium">Types</p>
        <p class="mt-1 text-xl font-semibold text-amber-400">{summary.types.toLocaleString()}</p>
      </div>
      <div class="rounded-lg bg-surface-z2 p-3">
        <p class="text-[10px] text-surface-z6 uppercase tracking-wide font-medium">Edges</p>
        <p class="mt-1 text-xl font-semibold text-surface-z8">{graphEdges.length.toLocaleString()}</p>
      </div>
      <div class="rounded-lg bg-surface-z2 p-3">
        <p class="text-[10px] text-surface-z6 uppercase tracking-wide font-medium">Communities</p>
        <p class="mt-1 text-xl font-semibold text-surface-z8">{communities.length}</p>
      </div>
      <div class="rounded-lg bg-surface-z2 p-3">
        <p class="text-[10px] text-surface-z6 uppercase tracking-wide font-medium">Sessions</p>
        <p class="mt-1 text-xl font-semibold text-surface-z8">{sessions.length}</p>
      </div>
    </div>

    <!-- Stack & Libs -->
    {#if summary.stack.length > 0 || summary.libs.length > 0}
      <div class="space-y-2">
        {#if summary.stack.length > 0}
          <div class="flex items-center gap-2">
            <span class="text-[10px] text-surface-z4 uppercase w-12">Stack</span>
            <div class="flex flex-wrap gap-1">
              {#each summary.stack as tech}
                <span class="rounded px-1.5 py-0.5 text-[10px] bg-primary-z2 text-primary-z7">{tech}</span>
              {/each}
            </div>
          </div>
        {/if}
        {#if summary.libs.length > 0}
          <div class="flex items-center gap-2">
            <span class="text-[10px] text-surface-z4 uppercase w-12">Libs</span>
            <div class="flex flex-wrap gap-1">
              {#each summary.libs.slice(0, 15) as lib}
                <span class="rounded px-1.5 py-0.5 text-[10px] bg-surface-z3 text-surface-z5">{lib}</span>
              {/each}
              {#if summary.libs.length > 15}
                <span class="text-[10px] text-surface-z4">+{summary.libs.length - 15}</span>
              {/if}
            </div>
          </div>
        {/if}
      </div>
    {/if}

    <!-- Solution membership -->
    {#if summary.solutions && summary.solutions.length > 0}
      <div class="flex items-center gap-2">
        <span class="text-[10px] text-surface-z4">Part of:</span>
        {#each summary.solutions as sol}
          <a href="/s/{sol.solutionId}" class="rounded px-1.5 py-0.5 text-[10px] bg-accent-z2 text-accent-z7 hover:bg-accent-z3">{sol.solutionName} ({sol.role})</a>
        {/each}
      </div>
    {/if}

    <!-- Graph -->
    {#if graphNodes.length > 0}
      <div>
        <h3 class="text-xs font-semibold text-surface-z5 uppercase tracking-wide mb-2">Code Graph</h3>
        <div class="h-[480px] rounded-lg border border-surface-z0/30 overflow-hidden">
          <GraphCanvas nodes={graphNodes} edges={graphEdges} />
        </div>
        <p class="text-[10px] text-surface-z3 mt-1">{graphNodes.length} nodes · {graphEdges.length} edges</p>
      </div>
    {/if}

    <!-- High Complexity -->
    {#if highComplexityFns.length > 0}
      <div>
        <h3 class="text-xs font-semibold text-surface-z5 uppercase tracking-wide mb-2">High Complexity Functions</h3>
        <div class="rounded-lg border border-surface-z0/50 divide-y divide-surface-z0/30">
          {#each highComplexityFns as fn}
            <div class="flex items-center gap-3 px-3 py-2 text-xs">
              <span class="h-2 w-2 rounded-full shrink-0 {fn.complexity >= 30 ? 'bg-error-z5' : fn.complexity >= 15 ? 'bg-warning-z5' : 'bg-info-z5'}"></span>
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
        <h3 class="text-xs font-semibold text-surface-z5 uppercase tracking-wide mb-2">Code Communities</h3>
        <div class="grid grid-cols-2 xl:grid-cols-3 gap-2">
          {#each communities.slice(0, 12) as c}
            <div class="rounded-lg bg-surface-z2 px-3 py-2">
              <div class="flex items-center gap-2">
                <span class="rounded px-1.5 py-0.5 text-[10px] font-medium bg-primary-z2 text-primary-z7">{c.size}</span>
                <span class="text-[10px] text-surface-z4">#{c.id}</span>
              </div>
              <p class="text-xs text-surface-z6 mt-1 truncate">{c.sample_members.join(', ')}</p>
            </div>
          {/each}
        </div>
      </div>
    {/if}

    <!-- Sessions -->
    <div>
      <h3 class="text-xs font-semibold text-surface-z5 uppercase tracking-wide mb-2">Sessions</h3>
      {#if sessions.length > 0}
        {@const totalCost = sessions.reduce((sum, s) => sum + (s.cost ?? 0), 0)}
        {#if totalCost > 0}
          <p class="text-[10px] text-surface-z4 mb-2">{sessions.length} sessions · ${totalCost.toFixed(2)} total cost</p>
        {/if}
        <div class="space-y-1">
          {#each sessions.slice(0, 10) as s}
            <div class="flex items-center gap-3 rounded-lg bg-surface-z2 px-3 py-2 text-sm">
              <span class="flex-1 truncate text-surface-z7">{s.task}</span>
              {#if s.cost}
                <span class="text-[10px] text-surface-z4">${s.cost.toFixed(2)}</span>
              {/if}
              <span class="text-[10px] {ftrClass(s.ftr)}">
                {s.ftr != null ? `${Math.round(s.ftr * 100)}%` : s.outcome ?? '…'}
              </span>
            </div>
          {/each}
        </div>
      {:else}
        <p class="text-xs text-surface-z4 py-3 text-center">No sessions yet. Sessions appear when you use sensei MCP tools in your editor.</p>
      {/if}
    </div>

    <!-- Doc Drift -->
    {#if docDrift.length > 0}
      <div>
        <h3 class="text-xs font-semibold text-surface-z5 uppercase tracking-wide mb-2">Doc Drift</h3>
        <div class="space-y-1">
          {#each docDrift as d}
            <div class="flex items-center gap-2 rounded-lg bg-warning-z1 px-3 py-2 text-xs">
              <span class="text-warning-z6">{d.edge_type}</span>
              <span class="text-surface-z6 flex-1 truncate">{d.doc_path.split('/').slice(-2).join('/')}</span>
            </div>
          {/each}
        </div>
      </div>
    {/if}

  {/if}
</div>
