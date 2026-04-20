<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { getPort } from '$lib/appstate.svelte.js';
  import { getRepoStore } from '$lib/repos.svelte.js';
  import { FolderInput, RepoListItem, StatusBadge } from '$lib/components/index.js';
  import { ftrColorClass, ftrFormat } from '$lib/components/colors.js';

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
        {#if store.anyIndexing}
          &middot; <span class="text-primary-z6">indexing in progress</span>
        {/if}
      </p>
    </div>
    <button onclick={() => showAddFolder = !showAddFolder}
      class="rounded-lg bg-primary-z2 px-3 py-1.5 text-xs font-medium text-primary-z7 hover:bg-primary-z3">
      {showAddFolder ? 'Done' : '+ Add folder'}
    </button>
  </div>

  <!-- Add folder (toggle) -->
  {#if showAddFolder}
    <FolderInput onadd={(path) => store.scanFolder(path)} scanning={store.anyIndexing} />
  {/if}

  <!-- Aggregate progress (when indexing) -->
  {#if store.anyIndexing}
    <div class="rounded-lg bg-surface-z2/50 border border-surface-z0/30 p-3 space-y-1.5">
      <div class="flex items-center justify-between text-xs">
        <span class="text-surface-z5">{store.indexingCount} repos indexing</span>
        <span class="text-surface-z4">
          {store.completedFiles.toLocaleString()} / {store.totalFiles.toLocaleString()} files
        </span>
      </div>
      <div class="h-1.5 rounded-full bg-surface-z3 overflow-hidden">
        <div class="h-full rounded-full bg-primary-z5 transition-all"
          style="width: {store.totalFiles > 0 ? (store.completedFiles / store.totalFiles) * 100 : 0}%"></div>
      </div>
    </div>
  {/if}

  <!-- Search -->
  {#if store.totalCount > 5}
    <input
      type="text"
      bind:value={store.search}
      placeholder="Filter projects..."
      class="w-full rounded-lg border border-surface-z3 bg-surface-z2 px-3 py-1.5 text-sm text-surface-z7 outline-none placeholder:text-surface-z4 focus:border-primary-z4"
    />
  {/if}

  <!-- Empty state -->
  {#if store.totalCount === 0}
    <div class="rounded-lg border border-dashed border-surface-z3 p-8 text-center space-y-3">
      <p class="text-base font-medium text-surface-z7">No projects yet</p>
      <p class="text-xs text-surface-z4 max-w-sm mx-auto">
        Add a folder to scan for repositories, or run the setup wizard.
      </p>
      <div class="flex items-center justify-center gap-3 pt-2">
        <button onclick={() => showAddFolder = true}
          class="rounded-lg bg-primary-z2 px-3 py-1.5 text-xs font-medium text-primary-z7 hover:bg-primary-z3">
          Add folder
        </button>
        <a href="/setup" class="rounded-lg bg-surface-z2 px-3 py-1.5 text-xs font-medium text-surface-z6 hover:bg-surface-z3">
          Setup wizard
        </a>
      </div>
    </div>

  <!-- Project list -->
  {:else}
    <div class="space-y-1">
      {#each store.repos as repo (repo.project.repo_id)}
        <RepoListItem
          {repo}
          href="/p/{repo.project.repo_id}"
          onexclude={(id) => store.excludeRepo(id)}
        />
      {/each}

      {#if store.repos.length === 0 && store.search}
        <p class="text-xs text-surface-z4 text-center py-4">No projects match "{store.search}"</p>
      {/if}
    </div>
  {/if}

</div>
