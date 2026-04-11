<script lang="ts">
  import { page } from '$app/stores';
  import { onMount } from 'svelte';
  import { getSolutionById, updateSolution } from '$lib/solutions.js';
  import { senseiApi } from '$lib/api.js';
  import RepoCard from '$lib/RepoCard.svelte';
  import type { ServerProject, RepoRole } from '$lib/types.js';

  let solution = $derived(getSolutionById($page.params.id as string));
  let port = $state(parseInt(localStorage.getItem('sensei:port') ?? '7744', 10));

  let serverProjects = $state<ServerProject[]>([]);
  let expandedPath = $state<string | null>(null);

  function toggleExpanded(path: string) {
    expandedPath = expandedPath === path ? null : path;
  }

  function handleRoleChange(path: string, role: RepoRole) {
    if (!solution) return;
    const updatedRepos = solution.repos.map(r =>
      r.path === path ? { ...r, role } : r
    );
    updateSolution(solution.id, { repos: updatedRepos });
  }

  async function indexAll() {
    if (!solution) return;
    const api = senseiApi(port);
    for (const repo of solution.repos) {
      await api.registerProject(repo.repoId, repo.label ?? repo.path.split('/').at(-1) ?? repo.repoId, repo.path);
      await api.indexRepo(repo.repoId, repo.path);
    }
  }

  async function load() {
    serverProjects = await senseiApi(port).getProjects();
  }

  onMount(() => {
    load();
    const interval = setInterval(load, 10_000);
    return () => clearInterval(interval);
  });
</script>

{#if solution}
  <div class="flex-1 overflow-y-auto px-5 py-4 space-y-3">

    <div class="flex items-center justify-between mb-2">
      <p class="text-xs text-surface-z4">{solution.repos.length} repo{solution.repos.length === 1 ? '' : 's'}</p>
      <button
        onclick={indexAll}
        class="rounded-md bg-primary-z2 px-3 py-1 text-xs font-medium text-primary-z7 hover:bg-primary-z3 transition-colors"
      >
        Index all
      </button>
    </div>

    {#each solution.repos as repo (repo.path)}
      <RepoCard
        {repo}
        serverInfo={serverProjects.find(p => p.repoId === repo.repoId)}
        {port}
        expanded={expandedPath === repo.path}
        onToggle={() => toggleExpanded(repo.path)}
        onRoleChange={(role) => handleRoleChange(repo.path, role)}
      />
    {/each}

  </div>
{/if}
