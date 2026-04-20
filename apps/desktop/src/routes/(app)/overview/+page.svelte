<script lang="ts">
  import { onMount } from 'svelte';
  import { senseiApi } from '$lib/api.js';
  import { getPort } from '$lib/appstate.svelte.js';
  import { getSolutions } from '$lib/solutions.svelte.js';
  import type { ServerProject, Solution } from '$lib/types.js';
  import type { OverviewData, SolutionSummary, ScopeMetrics } from '$lib/observatory/types.js';

  let loading = $state(true);
  let projects = $state<ServerProject[]>([]);
  let solutions = $state<Solution[]>([]);
  let metrics = $state<Record<string, any>>({});

  onMount(async () => {
    const api = senseiApi(getPort());
    const [projs, sessions] = await Promise.all([
      api.getProjects(),
      api.getSessions(),
    ]);
    projects = projs;
    solutions = getSolutions();

    // Fetch metrics for each indexed project
    for (const p of projs.filter(p => p.indexed_at)) {
      const m = await api.getMetrics(p.repo_id).catch(() => null);
      if (m) metrics[p.repo_id] = m;
    }

    loading = false;
  });

  // Group projects by solution (standalone projects become single-project solutions)
  let solutionViews = $derived((): SolutionSummary[] => {
    const solRepoIds = new Set(solutions.flatMap(s => s.repos.map(r => r.repoId)));
    const result: SolutionSummary[] = [];

    // Real solutions
    for (const sol of solutions) {
      const solProjects = sol.repos.map(r => {
        const proj = projects.find(p => p.repo_id === r.repoId);
        return {
          id: r.repoId,
          name: proj?.name ?? r.repoId,
          role: r.role,
          sourceType: 'git' as const,
          state: proj?.indexed_at ? 'active' as const : 'inactive' as const,
          indexedAt: proj?.indexed_at,
        };
      });
      result.push({
        id: sol.id, name: sol.name, description: sol.description,
        projects: solProjects,
        state: solProjects.some(p => p.state === 'active') ? 'active' : 'inactive',
        metrics: aggregateMetrics(sol.repos.map(r => r.repoId)),
      });
    }

    // Standalone projects (not in any solution)
    for (const p of projects.filter(pr => !solRepoIds.has(pr.repo_id))) {
      result.push({
        id: p.repo_id, name: p.name, description: p.path,
        projects: [{ id: p.repo_id, name: p.name, role: 'monorepo', sourceType: 'git', state: p.indexed_at ? 'active' : 'inactive', indexedAt: p.indexed_at }],
        state: p.indexed_at ? 'active' : 'inactive',
        metrics: aggregateMetrics([p.repo_id]),
      });
    }

    return result;
  });

  function aggregateMetrics(repoIds: string[]): ScopeMetrics {
    let sessionCount = 0;
    let ftrSum = 0; let ftrCount = 0;
    let turnCount = 0; let revisionCount = 0;
    for (const id of repoIds) {
      const m = metrics[id];
      if (!m) continue;
      sessionCount += m.session_count ?? 0;
      if (m.ftr != null) { ftrSum += m.ftr; ftrCount++; }
      turnCount += m.turn_count ?? 0;
      revisionCount += m.revision_count ?? 0;
    }
    const ftr = ftrCount > 0 ? ftrSum / ftrCount : 0;
    const rework = turnCount > 0 ? revisionCount / turnCount : 0;
    return {
      period: { label: 'All time', from: '', to: '' },
      ftr: { value: ftr, quality: ftrCount > 0 ? 'exact' : 'unavailable' },
      sessionCount: { value: sessionCount, quality: 'exact' },
      reworkRate: { value: rework, quality: turnCount > 0 ? 'exact' : 'unavailable' },
      tokens: { value: 0, quality: 'unavailable', trackingUrl: 'https://github.com/anthropics/claude-code/issues/11008' },
      cost: { value: 0, quality: 'unavailable', trackingUrl: 'https://github.com/anthropics/claude-code/issues/11008' },
      toolAdherence: { mcp: 0, fallback: 0, total: 0 },
    };
  }

  function stateCls(state: string): string {
    if (state === 'active') return 'bg-success-z2 text-success-z7';
    if (state === 'recent') return 'bg-info-z2 text-info-z7';
    if (state === 'inactive') return 'bg-surface-z3 text-surface-z5';
    return 'bg-surface-z3 text-surface-z4';
  }

  function ftrColor(v: number): string {
    if (v >= 0.8) return 'text-success-z6';
    if (v >= 0.5) return 'text-warning-z6';
    return 'text-error-z6';
  }

  function pctFmt(v: number): string { return `${Math.round(v * 100)}%`; }
</script>

<div class="h-full overflow-y-auto px-6 py-5 space-y-6">

  <div>
    <h2 class="text-lg font-semibold text-surface-z8">Overview</h2>
    <p class="text-xs text-surface-z4">
      {#if loading}Loading...
      {:else}{projects.length} projects &middot; {solutions.length} solutions
      {/if}
    </p>
  </div>

  {#if loading}
    <div class="flex items-center justify-center py-12">
      <p class="text-sm text-surface-z4">Loading projects...</p>
    </div>

  {:else if projects.length === 0}
    <!-- Empty state: first time, no projects yet -->
    <div class="rounded-lg border border-dashed border-surface-z3 p-8 text-center space-y-3">
      <p class="text-base font-medium text-surface-z7">No projects yet</p>
      <p class="text-xs text-surface-z4 max-w-sm mx-auto">
        Scan a folder to discover your repositories, or the setup wizard will guide you through the process.
      </p>
      <div class="flex items-center justify-center gap-3 pt-2">
        <a href="/all" class="rounded-md bg-primary-z2 px-3 py-1.5 text-xs font-medium text-primary-z7 hover:bg-primary-z3">
          Go to Projects
        </a>
        <a href="/setup" class="rounded-md bg-surface-z2 px-3 py-1.5 text-xs font-medium text-surface-z6 hover:bg-surface-z3">
          Setup Wizard
        </a>
      </div>
    </div>

  {:else}
    <!-- Global metrics (only if we have any sessions) -->
    {@const globalMetrics = aggregateMetrics(projects.map(p => p.repo_id))}
    {#if (globalMetrics.sessionCount.value as number) > 0}
      <div class="grid grid-cols-4 gap-3">
        {#each [
          { label: 'FTR', value: globalMetrics.ftr.quality !== 'unavailable' ? pctFmt(globalMetrics.ftr.value as number) : '—' },
          { label: 'Sessions', value: String(globalMetrics.sessionCount.value) },
          { label: 'Rework', value: globalMetrics.reworkRate.quality !== 'unavailable' ? pctFmt(globalMetrics.reworkRate.value as number) : '—' },
          { label: 'Projects', value: `${projects.filter(p => p.indexed_at).length} indexed` },
        ] as card}
          <div class="rounded-lg bg-surface-z2 p-3">
            <p class="text-[10px] text-surface-z5 uppercase tracking-wide font-medium">{card.label}</p>
            <p class="mt-1 text-xl font-semibold {card.value === '—' ? 'text-surface-z3' : 'text-primary-z6'}">{card.value}</p>
          </div>
        {/each}
      </div>
    {:else}
      <!-- No sessions yet but projects exist -->
      <div class="rounded-lg bg-surface-z2/50 border border-surface-z0/30 p-4">
        <p class="text-xs text-surface-z5">
          {projects.length} projects found &middot; {projects.filter(p => p.indexed_at).length} indexed &middot;
          Start a Claude Code session in any project to begin tracking metrics.
        </p>
      </div>
    {/if}

    <!-- Solutions / project list -->
    <div class="space-y-2">
      <p class="text-xs font-semibold uppercase tracking-wide text-surface-z5">
        {solutions.length > 0 ? 'Solutions & Projects' : 'Projects'}
      </p>

      {#each solutionViews() as sol (sol.id)}
        <a href="/s/{sol.id}" class="block rounded-lg bg-surface-z2 px-4 py-3 hover:bg-surface-z3/60 transition-colors">
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-3">
              <div class="flex h-8 w-8 items-center justify-center rounded-lg bg-primary-z3 text-sm font-bold text-primary-z7">
                {sol.name.charAt(0).toUpperCase()}
              </div>
              <div>
                <p class="text-sm font-medium text-surface-z7">{sol.name}</p>
                <p class="text-[10px] text-surface-z4">
                  {sol.projects.length} {sol.projects.length === 1 ? 'project' : 'projects'}
                  {#if sol.description} &middot; {sol.description}{/if}
                </p>
              </div>
            </div>
            <span class="rounded px-1.5 py-0.5 text-[10px] font-medium {stateCls(sol.state)}">{sol.state}</span>
          </div>

          {#if (sol.metrics.sessionCount.value as number) > 0}
            <div class="mt-2 grid grid-cols-3 gap-4 text-[10px]">
              <div>
                <span class="text-surface-z4">FTR</span>
                {#if sol.metrics.ftr.quality !== 'unavailable'}
                  <span class="ml-1 font-medium {ftrColor(sol.metrics.ftr.value as number)}">{pctFmt(sol.metrics.ftr.value as number)}</span>
                {:else}
                  <span class="ml-1 text-surface-z3">&mdash;</span>
                {/if}
              </div>
              <div>
                <span class="text-surface-z4">Sessions</span>
                <span class="ml-1 text-surface-z6">{sol.metrics.sessionCount.value}</span>
              </div>
              <div>
                <span class="text-surface-z4">Rework</span>
                {#if sol.metrics.reworkRate.quality !== 'unavailable'}
                  <span class="ml-1 text-surface-z6">{pctFmt(sol.metrics.reworkRate.value as number)}</span>
                {:else}
                  <span class="ml-1 text-surface-z3">&mdash;</span>
                {/if}
              </div>
            </div>
          {/if}
        </a>
      {/each}
    </div>
  {/if}

</div>
