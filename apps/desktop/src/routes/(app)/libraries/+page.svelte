<script lang="ts">
  import { onMount } from 'svelte';
  import { senseiApi } from '$lib/api.js';
  import type { LibEntry, LibDoc, DepVersion } from '$lib/types.js';

  let port = $state(parseInt(localStorage.getItem('sensei:port') ?? '7744', 10));
  let loading = $state(true);

  // Library data
  let allLibs = $state<LibEntry[]>([]);
  let filter = $state<'all' | 'shared' | 'external'>('all');
  let search = $state('');
  let selectedLib = $state<string | null>(null);

  // Selected lib details
  let libDocs = $state<LibDoc[]>([]);
  let searchResults = $state<LibDoc[]>([]);

  // Remote indexing modal
  let showIndexModal = $state(false);
  let indexLibName = $state('');
  let indexUrl = $state('');
  let indexVersion = $state('');
  let indexing = $state(false);
  let indexResult = $state<string | null>(null);

  let filteredLibs = $derived(() => {
    let libs = allLibs;
    if (filter === 'shared') libs = libs.filter(l => l.repoCount > 1);
    if (search) {
      const q = search.toLowerCase();
      libs = libs.filter(l => l.name.toLowerCase().includes(q));
    }
    return libs;
  });

  async function loadLibs() {
    const api = senseiApi(port);
    // Always fetch all libs — filter client-side
    const result = await api.getLibs();
    allLibs = result.libs;
    loading = false;
  }

  async function selectLib(name: string) {
    selectedLib = name;
    const api = senseiApi(port);
    libDocs = await api.getLibDocs(name);
  }

  async function searchDocs() {
    if (!search) { searchResults = []; return; }
    const api = senseiApi(port);
    searchResults = await api.searchLibDocs(search);
  }

  async function indexRemoteLib() {
    if (!indexLibName || !indexUrl) return;
    indexing = true;
    indexResult = null;
    const api = senseiApi(port);
    const result = await api.indexLib(indexLibName, indexUrl, indexVersion || undefined);
    if (result.ok) {
      indexResult = `Indexed ${(result as any).docsIndexed} docs from ${(result as any).sourceType}`;
      await loadLibs();
      selectLib(indexLibName);
    } else {
      indexResult = `Error: ${(result as any).error ?? 'Failed'}`;
    }
    indexing = false;
  }

  onMount(() => { loadLibs(); });
</script>

<div class="flex-1 overflow-y-auto px-6 py-5">

  <!-- Header -->
  <div class="flex items-center justify-between mb-5">
    <h2 class="text-lg font-semibold text-surface-z8">Libraries</h2>
    <button
      onclick={() => { showIndexModal = true; indexResult = null; }}
      class="rounded-lg bg-primary-z3 px-3 py-1.5 text-xs font-medium text-primary-z7 hover:bg-primary-z4"
    >
      + Index Remote Library
    </button>
  </div>

  <div class="flex gap-5" style="min-height: 60vh;">

    <!-- Left panel: lib list -->
    <div class="w-72 shrink-0 space-y-3">
      <!-- Search -->
      <input
        type="text"
        placeholder="Search libraries..."
        bind:value={search}
        oninput={() => searchDocs()}
        class="w-full rounded-lg border border-surface-z3 bg-surface-z1 px-3 py-2 text-sm text-surface-z7 outline-none focus:border-primary-z4"
      />

      <!-- Filters -->
      <div class="flex gap-1">
        {#each [['all', 'All'], ['shared', 'Shared']] as [val, label]}
          <button
            onclick={() => { filter = val as any; loadLibs(); }}
            class="rounded px-2 py-1 text-[10px] font-medium {filter === val ? 'bg-primary-z3 text-primary-z7' : 'bg-surface-z2 text-surface-z5 hover:bg-surface-z3'}"
          >{label}</button>
        {/each}
      </div>

      <!-- Search results from lib docs -->
      {#if searchResults.length > 0}
        <div class="space-y-1">
          <p class="text-[10px] text-surface-z4 uppercase">Doc Search Results</p>
          {#each searchResults.slice(0, 5) as doc}
            <div class="rounded-lg bg-accent-z2/50 px-2 py-1.5 cursor-pointer hover:bg-accent-z2" onclick={() => selectLib(doc.id.split(':')[1] ?? '')}>
              <p class="text-xs font-medium text-accent-z7 truncate">{doc.title}</p>
              <p class="text-[10px] text-surface-z4 truncate">{doc.summary}</p>
            </div>
          {/each}
        </div>
      {/if}

      <!-- Lib list -->
      <div class="space-y-1 max-h-[60vh] overflow-y-auto">
        {#if loading}
          <p class="text-xs text-surface-z4 py-4 text-center">Loading...</p>
        {:else if filteredLibs().length === 0}
          <p class="text-xs text-surface-z4 py-4 text-center">No libraries found</p>
        {:else}
          {#each filteredLibs() as lib}
            <button
              onclick={() => selectLib(lib.name)}
              class="w-full text-left rounded-lg px-3 py-2 {selectedLib === lib.name ? 'bg-primary-z2 border border-primary-z3' : 'bg-surface-z2 hover:bg-surface-z3'}"
            >
              <div class="flex items-center justify-between">
                <span class="text-sm font-medium text-surface-z7 truncate">{lib.name}</span>
                <span class="text-[10px] text-surface-z4">{lib.repoCount} repo{lib.repoCount !== 1 ? 's' : ''}</span>
              </div>
              {#if lib.repos.length > 0}
                <p class="text-[10px] text-surface-z4 truncate mt-0.5">{lib.repos.join(', ')}</p>
              {/if}
            </button>
          {/each}
          <p class="text-[10px] text-surface-z3 text-center pt-2">{filteredLibs().length} libraries</p>
        {/if}
      </div>
    </div>

    <!-- Right panel: lib detail -->
    <div class="flex-1 min-w-0">
      {#if selectedLib}
        {@const selectedLibInfo = allLibs.find(l => l.name === selectedLib)}
        <div class="space-y-4">
          <div class="flex items-center gap-3">
            <h3 class="text-base font-semibold text-surface-z8">{selectedLib}</h3>
            {#if selectedLibInfo}
              <span class="rounded px-1.5 py-0.5 text-[10px] bg-surface-z3 text-surface-z5">
                {selectedLibInfo.repoCount} repo{selectedLibInfo.repoCount !== 1 ? 's' : ''}
              </span>
            {/if}
          </div>

          <!-- Indexed docs -->
          {#if libDocs.length > 0}
            <div>
              <h4 class="text-xs font-semibold text-surface-z5 uppercase tracking-wide mb-2">Documentation ({libDocs.length})</h4>
              <div class="space-y-2">
                {#each libDocs as doc}
                  <div class="rounded-lg bg-surface-z2 p-3">
                    <div class="flex items-center justify-between mb-1">
                      <h5 class="text-sm font-medium text-surface-z7">{doc.title}</h5>
                      <span class="text-[10px] text-surface-z4">{doc.source_type}</span>
                    </div>
                    <p class="text-xs text-surface-z5 line-clamp-3">{doc.summary}</p>
                    {#if doc.url}
                      <a href={doc.url} target="_blank" rel="noopener" class="text-[10px] text-primary-z6 hover:text-primary-z7 mt-1 inline-block">Source</a>
                    {/if}
                  </div>
                {/each}
              </div>
            </div>
          {:else}
            <div class="rounded-lg bg-surface-z2 p-4 text-center">
              <p class="text-sm text-surface-z5">No indexed documentation</p>
              <p class="text-xs text-surface-z4 mt-1">Use "Index Remote Library" to add docs from a URL</p>
            </div>
          {/if}

          <!-- Used by repos -->
          {#if selectedLibInfo && selectedLibInfo.repos.length > 0}
            <div>
              <h4 class="text-xs font-semibold text-surface-z5 uppercase tracking-wide mb-2">Used by</h4>
              <div class="flex flex-wrap gap-1">
                {#each selectedLibInfo.repos as repo}
                  <span class="rounded px-2 py-1 text-xs bg-surface-z2 text-surface-z6">{repo}</span>
                {/each}
              </div>
            </div>
          {/if}
        </div>
      {:else}
        <div class="flex items-center justify-center h-full text-surface-z4">
          <p class="text-sm">Select a library to view details</p>
        </div>
      {/if}
    </div>

  </div>
</div>

<!-- Index Remote Library Modal -->
{#if showIndexModal}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/40" onclick={() => showIndexModal = false}>
    <div class="bg-surface-z1 rounded-xl shadow-xl p-6 w-full max-w-md space-y-4" onclick={(e) => e.stopPropagation()}>
      <h3 class="text-base font-semibold text-surface-z8">Index Remote Library</h3>
      <p class="text-xs text-surface-z4">Fetch documentation from a URL (llms.txt, README, or docs page) and store it locally for MCP queries.</p>

      <div class="space-y-3">
        <div>
          <label class="text-xs text-surface-z5 mb-1 block">Library Name</label>
          <input bind:value={indexLibName} placeholder="e.g. bits-ui" class="w-full rounded-lg border border-surface-z3 bg-surface-z1 px-3 py-2 text-sm text-surface-z7 outline-none focus:border-primary-z4" />
        </div>
        <div>
          <label class="text-xs text-surface-z5 mb-1 block">Documentation URL</label>
          <input bind:value={indexUrl} placeholder="https://example.com/llms.txt" class="w-full rounded-lg border border-surface-z3 bg-surface-z1 px-3 py-2 text-sm text-surface-z7 outline-none focus:border-primary-z4" />
        </div>
        <div>
          <label class="text-xs text-surface-z5 mb-1 block">Version (optional)</label>
          <input bind:value={indexVersion} placeholder="1.0.0" class="w-full rounded-lg border border-surface-z3 bg-surface-z1 px-3 py-2 text-sm text-surface-z7 outline-none focus:border-primary-z4" />
        </div>
      </div>

      {#if indexResult}
        <p class="text-xs {indexResult.startsWith('Error') ? 'text-error-z6' : 'text-success-z6'}">{indexResult}</p>
      {/if}

      <div class="flex gap-2 justify-end">
        <button onclick={() => showIndexModal = false} class="rounded-lg px-3 py-1.5 text-xs text-surface-z5 hover:bg-surface-z2">Cancel</button>
        <button
          onclick={indexRemoteLib}
          disabled={indexing || !indexLibName || !indexUrl}
          class="rounded-lg bg-primary-z3 px-4 py-1.5 text-xs font-medium text-primary-z7 hover:bg-primary-z4 disabled:opacity-50"
        >
          {indexing ? 'Indexing...' : 'Index'}
        </button>
      </div>
    </div>
  </div>
{/if}
