<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { getPort } from '$lib/appstate.svelte.js';
  import { getRepoStore } from '$lib/repos.svelte.js';
  import { FolderInput, RepoListItem } from '$lib/components/index.js';

  const store = getRepoStore(getPort());
  onMount(() => store.connect());
  onDestroy(() => store.disconnect());

  let showAddFolder = $state(false);
</script>

<div class="h-full overflow-y-auto px-6 py-5 space-y-5">

  <!-- Header -->
  <div class="flex items-center justify-between">
    <div>
      <h2 class="text-lg font-semibold text-surface-z8">Overview</h2>
      <p class="text-xs text-surface-z4">
        {store.totalCount} projects &middot; {store.indexedCount} indexed
        {#if store.solutionCount > 0} &middot; {store.solutionCount} solutions{/if}
        {#if store.anyIndexing}
          &middot; <span class="text-primary-z6">indexing</span>
        {/if}
      </p>
    </div>
    <button onclick={() => showAddFolder = !showAddFolder}
      class="rounded-lg bg-primary-z2 px-3 py-1.5 text-xs font-medium text-primary-z7 hover:bg-primary-z3">
      {showAddFolder ? 'Done' : '+ Add folder'}
    </button>
  </div>

  <!-- Add folder -->
  {#if showAddFolder}
    <FolderInput onadd={(path) => store.scanFolder(path)} scanning={store.anyIndexing} />
  {/if}

  <!-- Aggregate progress -->
  {#if store.anyIndexing}
    <div class="rounded-lg bg-surface-z2/50 border border-surface-z0/30 p-3 space-y-1.5">
      <div class="flex items-center justify-between text-xs">
        <span class="text-surface-z5">{store.indexingCount} repos indexing</span>
        <span class="text-surface-z4">{store.completedFiles.toLocaleString()} / {store.totalFiles.toLocaleString()} files</span>
      </div>
      <div class="h-1.5 rounded-full bg-surface-z3 overflow-hidden">
        <div class="h-full rounded-full bg-primary-z5 transition-all"
          style="width: {store.totalFiles > 0 ? (store.completedFiles / store.totalFiles) * 100 : 0}%"></div>
      </div>
    </div>
  {/if}

  <!-- Search -->
  {#if store.totalCount > 5}
    <input type="text" bind:value={store.search} placeholder="Filter projects..."
      class="w-full rounded-lg border border-surface-z3 bg-surface-z2 px-3 py-1.5 text-sm text-surface-z7 outline-none placeholder:text-surface-z4 focus:border-primary-z4" />
  {/if}

  <!-- Empty state -->
  {#if store.totalCount === 0}
    <div class="rounded-lg border border-dashed border-surface-z3 p-8 text-center space-y-3">
      <p class="text-base font-medium text-surface-z7">No projects yet</p>
      <p class="text-xs text-surface-z4 max-w-sm mx-auto">Add a folder to scan for repositories.</p>
      <button onclick={() => showAddFolder = true}
        class="rounded-lg bg-primary-z2 px-3 py-1.5 text-xs font-medium text-primary-z7 hover:bg-primary-z3">
        Add folder
      </button>
    </div>

  {:else}
    <!-- Solutions (grouped) -->
    {#each store.enrichedSolutions as sol (sol.id)}
      <div class="space-y-1">
        <a href="/s/{sol.id}" class="flex items-center gap-2 px-1 group">
          <div class="flex h-6 w-6 items-center justify-center rounded-md bg-primary-z3 text-[10px] font-bold text-primary-z7">
            {sol.name.charAt(0).toUpperCase()}
          </div>
          <div class="flex-1 min-w-0">
            <span class="text-xs font-semibold text-surface-z6 group-hover:text-primary-z6">{sol.name}</span>
            <span class="text-[10px] text-surface-z4 ml-1.5">{sol.repos.length} repos &middot; {sol.description}</span>
          </div>
        </a>
        {#each sol.repos as repo (repo.project.repo_id)}
          <div class="ml-8">
            <RepoListItem {repo} href="/s/{sol.id}/p/{repo.project.repo_id}" onexclude={(id) => store.excludeRepo(id)} />
          </div>
        {/each}
      </div>
    {/each}

    <!-- Standalone projects -->
    {#if store.standalone.length > 0}
      {#if store.solutionCount > 0}
        <p class="text-xs font-semibold uppercase tracking-wide text-surface-z5 pt-2">Standalone Projects</p>
      {/if}
      <div class="space-y-1">
        {#each store.standalone as repo (repo.project.repo_id)}
          <RepoListItem {repo} href="/p/{repo.project.repo_id}" onexclude={(id) => store.excludeRepo(id)} />
        {/each}
      </div>
    {/if}

    {#if store.repos.length === 0 && store.search}
      <p class="text-xs text-surface-z4 text-center py-4">No projects match "{store.search}"</p>
    {/if}
  {/if}

</div>
