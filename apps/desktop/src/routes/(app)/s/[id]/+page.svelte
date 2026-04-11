<script lang="ts">
  import { page } from '$app/stores';
  import { onMount } from 'svelte';
  import { getSolutionById, updateSolution } from '$lib/solutions.js';
  import { senseiApi } from '$lib/api.js';
  import type { Solution, ServerProject, SolutionCategory } from '$lib/types.js';

  let solution = $derived(getSolutionById($page.params.id));
  let port = $state(parseInt(localStorage.getItem('sensei:port') ?? '7744', 10));

  let serverProjects = $state<ServerProject[]>([]);
  let sessions = $state<Array<{ id: string; task: string; project: string; ftr?: number | null; startedAt: string; outcome?: string }>>([]);
  let loading = $state(true);

  // Derived stats
  let repoCount = $derived(solution?.repos.length ?? 0);
  let repoIds = $derived(new Set(solution?.repos.map(r => r.repoId) ?? []));
  let indexedCount = $derived(serverProjects.filter(p => repoIds.has(p.repoId) && p.indexedAt).length);
  let errorCount = $derived(serverProjects.filter(p => repoIds.has(p.repoId) && p.lastError).length);
  let recentSessions = $derived(
    sessions
      .filter(s => repoIds.has(s.project))
      .slice(0, 5)
  );

  const ROLE_CLS: Record<string, string> = {
    backend: 'bg-info-z2 text-info-z7',
    frontend: 'bg-primary-z2 text-primary-z7',
    mobile: 'bg-secondary-z2 text-secondary-z7',
    library: 'bg-warning-z2 text-warning-z7',
    infra: 'bg-surface-z3 text-surface-z6',
    docs: 'bg-surface-z3 text-surface-z5',
    shared: 'bg-accent-z2 text-accent-z7',
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
    loading = false;
  }

  function ftrClass(ftr: number | null | undefined): string {
    if (ftr == null) return 'text-surface-z4';
    if (ftr >= 0.8) return 'text-success-z6';
    if (ftr >= 0.5) return 'text-warning-z6';
    return 'text-error-z6';
  }

  onMount(() => { load(); });
</script>

{#if solution}
  <div class="flex-1 overflow-y-auto px-6 py-5 space-y-6">

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
    <div class="grid grid-cols-4 gap-4">
      <div class="rounded-lg bg-surface-z2 p-3">
        <p class="text-[10px] text-surface-z4 uppercase tracking-wide">Repos</p>
        <p class="mt-1 text-xl font-semibold text-surface-z8">{repoCount}</p>
      </div>
      <div class="rounded-lg bg-surface-z2 p-3">
        <p class="text-[10px] text-surface-z4 uppercase tracking-wide">Indexed</p>
        <p class="mt-1 text-xl font-semibold {indexedCount === repoCount ? 'text-success-z6' : 'text-warning-z6'}">{indexedCount}/{repoCount}</p>
      </div>
      <div class="rounded-lg bg-surface-z2 p-3">
        <p class="text-[10px] text-surface-z4 uppercase tracking-wide">Errors</p>
        <p class="mt-1 text-xl font-semibold {errorCount > 0 ? 'text-error-z6' : 'text-success-z6'}">{errorCount}</p>
      </div>
      <div class="rounded-lg bg-surface-z2 p-3">
        <p class="text-[10px] text-surface-z4 uppercase tracking-wide">Sessions</p>
        <p class="mt-1 text-xl font-semibold text-surface-z8">{recentSessions.length}</p>
      </div>
    </div>

    <!-- Repos overview -->
    <div>
      <div class="flex items-center justify-between mb-2">
        <h3 class="text-xs font-semibold text-surface-z5 uppercase tracking-wide">Repos</h3>
        <a href="/s/{solution.id}/repos" class="text-xs text-primary-z6 hover:text-primary-z7">Manage</a>
      </div>
      <div class="space-y-1.5">
        {#each solution.repos as repo}
          {@const serverInfo = serverProjects.find(p => p.repoId === repo.repoId)}
          <div class="flex items-center gap-3 rounded-lg bg-surface-z2 px-3 py-2">
            <span class="rounded px-1.5 py-0.5 text-[10px] font-medium {ROLE_CLS[repo.role] ?? ROLE_CLS.unknown}">
              {repo.role}
            </span>
            <span class="text-sm text-surface-z7 flex-1 truncate">{repo.label ?? repo.path.split('/').at(-1)}</span>
            {#if serverInfo?.indexedAt}
              <span class="text-[10px] text-success-z5">indexed</span>
            {:else if serverInfo?.lastError}
              <span class="text-[10px] text-error-z5">error</span>
            {:else}
              <span class="text-[10px] text-surface-z4">not indexed</span>
            {/if}
          </div>
        {/each}
      </div>
    </div>

    <!-- Recent sessions -->
    {#if recentSessions.length > 0}
      <div>
        <div class="flex items-center justify-between mb-2">
          <h3 class="text-xs font-semibold text-surface-z5 uppercase tracking-wide">Recent Sessions</h3>
          <a href="/s/{solution.id}/sessions" class="text-xs text-primary-z6 hover:text-primary-z7">View all</a>
        </div>
        <div class="space-y-1">
          {#each recentSessions as s}
            <div class="flex items-center gap-3 rounded-lg bg-surface-z2 px-3 py-2 text-sm">
              <span class="flex-1 truncate text-surface-z7">{s.task}</span>
              <span class="text-[10px] {ftrClass(s.ftr)}">
                {s.ftr != null ? `${Math.round(s.ftr * 100)}%` : s.outcome ?? '…'}
              </span>
            </div>
          {/each}
        </div>
      </div>
    {/if}

  </div>
{/if}
