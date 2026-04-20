<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { getPort } from '$lib/appstate.svelte.js';
  import { getRepoStore } from '$lib/repos.svelte.js';
  import { RepoListItem } from '$lib/components/index.js';

  const store = getRepoStore(getPort());
  onMount(() => store.connect());
  onDestroy(() => store.disconnect());

  let inputValue = $state('');
  let inputMode = $derived(inputValue.startsWith('/') || inputValue.startsWith('~') ? 'add' : 'search');

  // Add-to-solution picker
  let addingRepoId = $state<string | null>(null);

  function startAddToSolution(repoId: string) {
    addingRepoId = addingRepoId === repoId ? null : repoId;
  }

  async function confirmAddToSolution(solutionId: string) {
    if (!addingRepoId) return;
    await store.addToSolution(solutionId, addingRepoId);
    addingRepoId = null;
  }

  function handleInput() {
    if (inputMode === 'add') {
      store.search = '';
    } else {
      store.search = inputValue;
    }
  }

  function handleSubmit() {
    if (inputMode === 'add' && inputValue.trim()) {
      store.scanFolder(inputValue.trim());
      inputValue = '';
    }
  }

  async function browse() {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const picked = await invoke<string | null>('pick_folder');
      if (picked) store.scanFolder(picked);
    } catch { /* browser preview */ }
  }
</script>

<div class="h-full overflow-y-auto px-6 py-5 space-y-5">

  <!-- Header with progress on the right -->
  <div class="flex items-start justify-between gap-4">
    <div>
      <h2 class="text-lg font-semibold text-surface-z8">Overview</h2>
      <p class="text-xs text-surface-z4">
        {store.totalCount} projects &middot; {store.indexedCount} indexed
        {#if store.solutionCount > 0} &middot; {store.solutionCount} solutions{/if}
      </p>
    </div>
    {#if store.anyIndexing}
      <div class="shrink-0 text-right">
        <p class="text-[10px] text-primary-z6 font-medium">{store.indexingCount} indexing</p>
        <div class="flex items-center gap-2 mt-0.5">
          <div class="w-24 h-1.5 rounded-full bg-surface-z3 overflow-hidden">
            <div class="h-full rounded-full bg-primary-z5 transition-all"
              style="width: {store.totalFiles > 0 ? (store.completedFiles / store.totalFiles) * 100 : 0}%"></div>
          </div>
          <span class="text-[10px] text-surface-z4">{store.completedFiles.toLocaleString()} / {store.totalFiles.toLocaleString()}</span>
        </div>
      </div>
    {/if}
  </div>

  <!-- Unified search / add bar -->
  <div class="flex gap-2">
    <div class="flex-1 relative">
      <span class="absolute left-2.5 top-1/2 -translate-y-1/2 text-sm {inputMode === 'add' ? 'i-solar-folder-add-bold-duotone text-primary-z5' : 'i-solar-magnifer-bold-duotone text-surface-z4'}"></span>
      <input
        type="text"
        bind:value={inputValue}
        oninput={handleInput}
        onkeydown={(e) => { if (e.key === 'Enter') handleSubmit(); }}
        placeholder="Search or type a path to scan (~/...)"
        class="w-full rounded-lg border bg-surface-z2 pl-8 pr-3 py-1.5 text-sm text-surface-z7 outline-none placeholder:text-surface-z4 {inputMode === 'add' ? 'border-primary-z4' : 'border-surface-z3 focus:border-primary-z4'}"
      />
    </div>
    {#if inputMode === 'add'}
      <button onclick={handleSubmit}
        class="rounded-lg bg-primary-z2 px-3 py-1.5 text-xs font-medium text-primary-z7 hover:bg-primary-z3">
        Scan
      </button>
    {/if}
    <button onclick={browse} title="Browse folder"
      class="rounded-lg border border-surface-z3 bg-surface-z2 px-2.5 text-surface-z5 hover:bg-surface-z3 transition-colors">
      <span class="i-solar-folder-open-bold-duotone text-sm"></span>
    </button>
  </div>

  <!-- Empty state -->
  {#if store.totalCount === 0}
    <div class="rounded-lg border border-dashed border-surface-z3 p-8 text-center space-y-3">
      <p class="text-base font-medium text-surface-z7">No projects yet</p>
      <p class="text-xs text-surface-z4 max-w-sm mx-auto">Add a folder to scan for repositories.</p>
      <button onclick={() => { inputValue = '~/'; }}
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
          <div>
            <RepoListItem
              {repo}
              href="/p/{repo.project.repo_id}"
              onexclude={(id) => store.excludeRepo(id)}
              onadd={store.solutionCount > 0 ? (id) => startAddToSolution(id) : undefined}
            />
            {#if addingRepoId === repo.project.repo_id && store.solutions.length > 0}
              <div class="ml-8 mt-1 flex items-center gap-2 rounded-lg bg-surface-z2/50 border border-primary-z3/30 px-3 py-2">
                <span class="text-[10px] text-surface-z5">Add to:</span>
                {#each store.solutions as sol (sol.id)}
                  <button
                    onclick={() => confirmAddToSolution(sol.id)}
                    class="rounded-md bg-primary-z2 px-2 py-0.5 text-[10px] font-medium text-primary-z7 hover:bg-primary-z3"
                  >{sol.name}</button>
                {/each}
                <button onclick={() => addingRepoId = null}
                  class="text-[10px] text-surface-z4 hover:text-surface-z6 ml-auto">cancel</button>
              </div>
            {/if}
          </div>
        {/each}
      </div>
    {/if}

    {#if store.repos.length === 0 && store.search}
      <p class="text-xs text-surface-z4 text-center py-4">No projects match "{store.search}"</p>
    {/if}
  {/if}

</div>
