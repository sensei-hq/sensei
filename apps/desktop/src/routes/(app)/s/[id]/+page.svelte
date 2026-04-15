<script lang="ts">
  import { getPort } from '$lib/appstate.svelte.js';
  import { page } from '$app/stores';
  import { onMount } from 'svelte';
  import { getSolutionById, updateSolution } from '$lib/solutions.svelte.js';
  import { senseiApi } from '$lib/api.js';
  import GraphCanvas from '$lib/GraphCanvas.svelte';
  import type { Solution, ServerProject, SolutionCategory, ProjectSummary, SolutionAnalysis, InferredRole, GraphNode, GraphEdge } from '$lib/types.js';

  let solution = $derived(getSolutionById($page.params.id as string));
  let port = $derived(getPort());

  let serverProjects = $state<ServerProject[]>([]);
  let sessions = $state<Array<{ id: string; task: string; project: string; ftr?: number | null; startedAt: string; outcome?: string; cost?: number }>>([]);
  let loading = $state(true);

  // Graph data for mini preview
  let graphNodes = $state<GraphNode[]>([]);
  let graphEdges = $state<GraphEdge[]>([]);

  // Cross-repo analysis
  let analysis = $state<SolutionAnalysis | null>(null);
  let inferredRoles = $state<InferredRole[]>([]);
  let repoSummaries = $state<Map<string, ProjectSummary>>(new Map());

  // Derived stats — subtrees count as indexed if parent is indexed
  let repoCount = $derived(solution?.repos.length ?? 0);
  let indexedCount = $derived(() => {
    if (!solution) return 0;
    let count = 0;
    for (const repo of solution.repos) {
      const isSubtree = repo.role === 'subtree';
      const parentId = isSubtree ? repo.repoId.split(':')[0] : repo.repoId;
      const info = serverProjects.find(p => p.repoId === repo.repoId)
        ?? (isSubtree ? serverProjects.find(p => p.repoId === parentId) : undefined);
      if (info?.indexedAt) count++;
    }
    return count;
  });
  let totalFunctions = $derived([...repoSummaries.values()].reduce((sum, s) => sum + s.functions, 0));
  let totalTypes = $derived([...repoSummaries.values()].reduce((sum, s) => sum + s.types, 0));
  let repoIds = $derived(new Set(solution?.repos.map(r => r.repoId) ?? []));
  let recentSessions = $derived(
    sessions
      .filter(s => repoIds.has(s.project))
      .slice(0, 5)
  );
  let totalCost = $derived(recentSessions.reduce((sum, s) => sum + (s.cost ?? 0), 0));

  const ROLE_CLS: Record<string, string> = {
    parent: 'bg-primary-z2 text-primary-z7',
    subtree: 'bg-accent-z2 text-accent-z7',
    api: 'bg-info-z2 text-info-z7',
    ui: 'bg-primary-z2 text-primary-z7',
    mobile: 'bg-secondary-z2 text-secondary-z7',
    'shared-lib': 'bg-warning-z2 text-warning-z7',
    infra: 'bg-surface-z3 text-surface-z6',
    docs: 'bg-surface-z3 text-surface-z5',
    unknown: 'bg-surface-z3 text-surface-z5',
  };

  const CATEGORY_OPTIONS: { value: SolutionCategory; label: string }[] = [
    { value: 'active', label: 'Active' },
    { value: 'side', label: 'Side Project' },
    { value: 'idea', label: 'Idea' },
  ];

  async function load() {
    const api = senseiApi(port);
    const [projects, sessionData] = await Promise.all([
      api.getProjects(),
      api.getSessions(),
    ]);
    serverProjects = projects;
    sessions = sessionData.sessions;

    if (solution) {
      // Load solution graph (merged across repos)
      try {
        const sg = await api.getSolutionGraph(solution.id);
        if (sg?.graph) {
          graphNodes = sg.graph.nodes;
          graphEdges = sg.graph.edges;
        }
      } catch {
        const allNodes: GraphNode[] = [];
        const allEdges: GraphEdge[] = [];
        for (const repo of solution.repos) {
          const data = await api.getGraphNodes(repo.repoId);
          allNodes.push(...data.nodes);
          allEdges.push(...data.edges);
        }
        graphNodes = allNodes;
        graphEdges = allEdges;
      }

      // Load per-repo summaries — for subtrees, fetch parent summary
      const summaries = new Map<string, ProjectSummary>();
      const seen = new Set<string>();
      await Promise.all(solution.repos.map(async (repo) => {
        const isSubtree = repo.role === 'subtree';
        const fetchId = isSubtree ? repo.repoId.split(':')[0] : repo.repoId;
        if (seen.has(fetchId)) {
          // Reuse parent summary for subtree
          const parentSummary = summaries.get(fetchId);
          if (parentSummary) summaries.set(repo.repoId, parentSummary);
          return;
        }
        seen.add(fetchId);
        const s = await api.getProjectSummary(fetchId);
        if (s) {
          summaries.set(fetchId, s);
          if (isSubtree) summaries.set(repo.repoId, s);
        }
      }));
      repoSummaries = summaries;

      // Load cross-repo analysis and inferred roles
      const [a, roles] = await Promise.all([
        api.analyzeSolution(solution.id).catch(() => null),
        api.getSolutionRoles(solution.id).catch(() => []),
      ]);
      analysis = a;
      inferredRoles = roles;
    }

    loading = false;
  }

  function ftrClass(ftr: number | null | undefined): string {
    if (ftr == null) return 'text-surface-z4';
    if (ftr >= 0.8) return 'text-success-z6';
    if (ftr >= 0.5) return 'text-warning-z6';
    return 'text-error-z6';
  }

  function getInferredRole(repoId: string): InferredRole | undefined {
    return inferredRoles.find(r => r.repo_id === repoId);
  }

  // Repos panel
  let showReposPanel = $state(false);

  onMount(() => { load(); });
</script>

{#if solution}
  <div class="h-full overflow-y-auto px-6 py-5 space-y-6">

    <!-- Header -->
    <div class="flex items-start justify-between gap-4">
      <div class="flex items-center gap-3">
        <div class="flex h-10 w-10 items-center justify-center rounded-xl bg-primary-z3 text-lg font-bold text-primary-z7">
          {solution.name.charAt(0).toUpperCase()}
        </div>
        <div>
          <h2 class="text-lg font-semibold text-surface-z8">{solution.name}</h2>
          {#if solution.client}
            <p class="text-xs text-surface-z4">{solution.client}</p>
          {/if}
        </div>
      </div>
      <select
        value={solution.category}
        onchange={(e) => updateSolution(solution!.id, { category: (e.target as HTMLSelectElement).value as SolutionCategory })}
        class="rounded-md border border-surface-z3 bg-surface-z1 px-2 py-1 text-xs text-surface-z6"
      >
        {#each CATEGORY_OPTIONS as opt}
          <option value={opt.value}>{opt.label}</option>
        {/each}
      </select>
    </div>

    <!-- Stats grid -->
    <div class="grid grid-cols-5 gap-3">
      <button onclick={() => showReposPanel = true} class="rounded-lg bg-surface-z2 p-3 text-left hover:bg-surface-z3/80 transition-colors cursor-pointer">
        <p class="text-[10px] text-surface-z6 uppercase tracking-wide font-medium">Repos</p>
        <p class="mt-1 text-xl font-semibold text-primary-z6">{repoCount}</p>
      </button>
      <div class="rounded-lg bg-surface-z2 p-3">
        <p class="text-[10px] text-surface-z6 uppercase tracking-wide font-medium">Indexed</p>
        <p class="mt-1 text-xl font-semibold {indexedCount() === repoCount ? 'text-success-z6' : 'text-warning-z6'}">{indexedCount()}/{repoCount}</p>
      </div>
      <div class="rounded-lg bg-surface-z2 p-3">
        <p class="text-[10px] text-surface-z6 uppercase tracking-wide font-medium">Functions</p>
        <p class="mt-1 text-xl font-semibold text-indigo-400">{totalFunctions.toLocaleString()}</p>
      </div>
      <div class="rounded-lg bg-surface-z2 p-3">
        <p class="text-[10px] text-surface-z6 uppercase tracking-wide font-medium">Types</p>
        <p class="mt-1 text-xl font-semibold text-amber-400">{totalTypes.toLocaleString()}</p>
      </div>
      <div class="rounded-lg bg-surface-z2 p-3">
        <p class="text-[10px] text-surface-z6 uppercase tracking-wide font-medium">Sessions</p>
        <p class="mt-1 text-xl font-semibold text-surface-z8">{recentSessions.length}</p>
      </div>
    </div>

    <!-- Cross-repo analysis (multi-repo solutions) -->
    {#if analysis && (analysis.shared_libs.length > 0 || analysis.links.length > 0)}
      <div class="rounded-lg bg-surface-z2/50 border border-surface-z0/30 p-4 space-y-3">
        <h3 class="text-xs font-semibold text-surface-z6 uppercase tracking-wide">Cross-Repo Analysis</h3>

        {#if analysis.links.length > 0}
          <div class="space-y-1">
            {#each analysis.links as link}
              <div class="flex items-center gap-2 text-xs">
                <span class="font-medium text-surface-z7">{link.from_repo}</span>
                <span class="text-surface-z4">→</span>
                <span class="font-medium text-surface-z7">{link.to_repo}</span>
                <span class="rounded px-1.5 py-0.5 bg-accent-z2 text-accent-z7 text-[10px]">{link.link_type}</span>
                <span class="text-surface-z4 text-[10px]">{(link.strength * 100).toFixed(0)}%</span>
              </div>
            {/each}
          </div>
        {/if}

        {#if analysis.shared_libs.length > 0}
          <div>
            <p class="text-[10px] text-surface-z5 mb-1">{analysis.shared_libs.length} shared libraries</p>
            <div class="flex flex-wrap gap-1">
              {#each analysis.shared_libs.slice(0, 12) as lib}
                <span class="rounded px-1.5 py-0.5 bg-warning-z2 text-warning-z7 text-[10px]">{lib.name}</span>
              {/each}
              {#if analysis.shared_libs.length > 12}
                <span class="text-[10px] text-surface-z4">+{analysis.shared_libs.length - 12} more</span>
              {/if}
            </div>
          </div>
        {/if}
      </div>
    {/if}

    <!-- Code graph preview -->
    {#if graphNodes.length > 0}
      <div>
        <div class="flex items-center justify-between mb-2">
          <h3 class="text-xs font-semibold text-surface-z6 uppercase tracking-wide">Code Graph</h3>
          <a href="/s/{solution.id}/arch" class="text-xs text-primary-z6 hover:text-primary-z7">Full view →</a>
        </div>
        <div class="h-[calc(100vh-20rem)] min-h-72 rounded-lg border border-surface-z0/30 overflow-hidden">
          <GraphCanvas nodes={graphNodes} edges={graphEdges} />
        </div>
        <p class="text-[10px] text-surface-z4 mt-1">{graphNodes.length} nodes · {graphEdges.length} edges</p>
      </div>
    {/if}

    <!-- Recent sessions -->
    {#if recentSessions.length > 0}
      <div>
        <div class="flex items-center justify-between mb-2">
          <h3 class="text-xs font-semibold text-surface-z6 uppercase tracking-wide">Recent Sessions</h3>
          <div class="flex items-center gap-3">
            {#if totalCost > 0}
              <span class="text-[10px] text-surface-z4">${totalCost.toFixed(2)} total</span>
            {/if}
            <a href="/s/{solution.id}/sessions" class="text-xs text-primary-z6 hover:text-primary-z7">View all</a>
          </div>
        </div>
        <div class="space-y-1">
          {#each recentSessions as s}
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
      </div>
    {/if}

  </div>

  <!-- Repos slide-out panel (single source of truth for repos) -->
  {#if showReposPanel}
    <div class="fixed inset-0 z-50 flex justify-end" onclick={() => showReposPanel = false}>
      <div class="w-96 h-full bg-surface-z1 border-l border-surface-z0/50 shadow-xl overflow-y-auto" onclick={(e) => e.stopPropagation()}>
        <div class="px-4 py-3 border-b border-surface-z0/50 flex items-center justify-between">
          <h3 class="text-sm font-semibold text-surface-z8">{repoCount} Repos in {solution.name}</h3>
          <button onclick={() => showReposPanel = false} class="text-surface-z4 hover:text-surface-z6 text-sm">✕</button>
        </div>
        <div class="p-3 space-y-2">
          {#each solution.repos as repo}
            {@const repoName = repo.label || repo.path?.split('/').at(-1) || repo.repoId}
            {@const isSubtree = repo.role === 'subtree'}
            {@const parentId = isSubtree ? repo.repoId.split(':')[0] : repo.repoId}
            {@const summary = repoSummaries.get(repo.repoId)}
            {@const serverInfo = serverProjects.find(p => p.repoId === repo.repoId) ?? (isSubtree ? serverProjects.find(p => p.repoId === parentId) : undefined)}
            {@const inferred = getInferredRole(repo.repoId)}
            {@const displayRole = inferred && inferred.confidence > 0.5 ? inferred.role : repo.role}
            {@const linkTarget = isSubtree ? `/p/${parentId}` : `/p/${repo.repoId}`}
            <a href={linkTarget} class="rounded-lg bg-surface-z2 px-4 py-3 space-y-1.5 hover:bg-surface-z3/60 transition-colors block" onclick={() => showReposPanel = false}>
              <div class="flex items-center gap-2">
                <span class="flex h-6 w-6 items-center justify-center rounded-md bg-surface-z3 text-[10px] font-bold text-surface-z6">
                  {repoName.charAt(0).toUpperCase()}
                </span>
                <span class="text-sm font-medium text-surface-z8 flex-1 truncate">{repoName}</span>
                <span class="rounded px-1.5 py-0.5 text-[10px] font-medium {ROLE_CLS[displayRole] ?? ROLE_CLS.unknown}">{displayRole}</span>
              </div>
              {#if repo.path}
                <p class="text-[10px] text-surface-z3 font-mono truncate">{repo.path}</p>
              {/if}
              <div class="flex items-center gap-3 text-[10px]">
                {#if summary && !isSubtree}
                  <span class="text-surface-z5 font-mono">{summary.functions} fn · {summary.types} ty</span>
                {/if}
                {#if isSubtree}
                  <span class="text-surface-z5">subtree of {parentId}</span>
                  {#if serverInfo?.indexedAt}
                    <span class="text-success-z5">indexed via parent</span>
                  {/if}
                {:else if serverInfo?.indexedAt}
                  <span class="text-success-z5">indexed</span>
                {:else}
                  <span class="text-surface-z4">not indexed</span>
                {/if}
              </div>
              {#if !isSubtree && summary?.libs && summary.libs.length > 0}
                <div class="flex flex-wrap gap-1">
                  {#each summary.libs.slice(0, 6) as lib}
                    <span class="rounded px-1 py-0.5 text-[9px] bg-surface-z3 text-surface-z5">{lib}</span>
                  {/each}
                  {#if summary.libs.length > 6}
                    <span class="text-[9px] text-surface-z4">+{summary.libs.length - 6}</span>
                  {/if}
                </div>
              {/if}
            </a>
          {/each}
        </div>
      </div>
    </div>
  {/if}
{/if}
