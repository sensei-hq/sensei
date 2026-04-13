<script lang="ts">
  import { onMount } from 'svelte';
  import { getPort, getDismissedSuggestions, dismissSuggestion as dismissSuggestionState } from '$lib/appstate.svelte.js';
  import {
    getSolutions, createSolution, addRepoToSolution,
    removeRepoFromSolution, inferRepoRole,
  } from '$lib/solutions.svelte.js';
  import { senseiApi } from '$lib/api.js';
  import { connectSSE, getQueueStatus, getProgressForRepo, isIndexing, startIndex, refreshStatus } from '$lib/indexer.svelte.js';
  import type { ScannedRepo, Solution, SolutionRepo, ServerProject, IndexQueueStatus } from '$lib/types.js';

  let scannedRepos = $state<ScannedRepo[]>([]);
  let projects = $state<ServerProject[]>([]);
  let search = $state('');
  let scanRoot = $state('');
  let scanning = $state(false);
  let queueStatus = $derived(getQueueStatus());
  let indexedCount = $derived(projects.filter(p => p.indexed_at).length);
  let totalCount = $derived(projects.length);

  let solutions = $derived(getSolutions());

  // Suggest groupings for unassigned repos with shared name prefixes
  let suggestions = $derived(computeSuggestions());

  function computeSuggestions(): Array<{ name: string; paths: string[] }> {
    const unassigned = scannedRepos.filter(r => !getSolutionForPath(r.path));
    if (unassigned.length < 2) return [];

    // Group by name prefix (strip trailing -api, -ui, -web, -backend, -frontend, -shared, -common, -lib, -core, -app, -service, -server, -client)
    const prefixMap = new Map<string, string[]>();
    for (const r of unassigned) {
      const prefix = r.name.toLowerCase()
        .replace(/[-_](api|ui|web|backend|frontend|shared|common|lib|core|app|service|server|client|mobile|admin|docs)$/, '')
        .trim();
      if (prefix.length >= 2 && prefix !== r.name.toLowerCase()) {
        const list = prefixMap.get(prefix) ?? [];
        list.push(r.path);
        prefixMap.set(prefix, list);
      }
    }

    return [...prefixMap.entries()]
      .filter(([, paths]) => paths.length >= 2)
      .map(([prefix, paths]) => ({
        name: prefix.replace(/[-_]/g, ' ').replace(/\b\w/g, c => c.toUpperCase()),
        paths,
      }));
  }

  function acceptSuggestion(suggestion: { name: string; paths: string[] }) {
    const repos: SolutionRepo[] = suggestion.paths.map(path => {
      const r = scannedRepos.find(s => s.path === path);
      return {
        repoId: r?.repoId ?? path.replace(/^\//, ''),
        path,
        role: r ? inferRepoRole(r) : 'unknown' as const,
        label: r?.name ?? path.split('/').at(-1) ?? path,
      };
    });
    createSolution(suggestion.name, repos);
  }

  function dismissSuggestionHandler(suggestion: { name: string; paths: string[] }) {
    dismissSuggestionState(suggestion.name.toLowerCase());
  }

  let filtered = $derived(
    scannedRepos
      .filter(r => {
        if (!search) return true;
        const q = search.toLowerCase();
        return r.name.toLowerCase().includes(q) || r.path.toLowerCase().includes(q);
      })
      .sort((a, b) => {
        // Currently indexing first, then queued, then indexed, then not indexed
        const aId = a.repoId ?? a.name;
        const bId = b.repoId ?? b.name;
        const aIndexing = isIndexing(aId) ? 0 : 1;
        const bIndexing = isIndexing(bId) ? 0 : 1;
        if (aIndexing !== bIndexing) return aIndexing - bIndexing;
        const aIndexed = projects.find(p => p.repo_id === aId)?.indexed_at ? 1 : 0;
        const bIndexed = projects.find(p => p.repo_id === bId)?.indexed_at ? 1 : 0;
        if (aIndexed !== bIndexed) return aIndexed - bIndexed; // not-indexed first (to show what needs attention)
        return a.name.localeCompare(b.name);
      })
  );

  function getSolutionForPath(path: string): Solution | undefined {
    return solutions.find(s => s.repos.some(r => r.path === path));
  }

  // New solution from selection
  let selectedPaths = $state<Set<string>>(new Set());
  let newSolutionName = $state('');
  let showCreate = $state(false);

  function toggleSelect(path: string) {
    const next = new Set(selectedPaths);
    if (next.has(path)) next.delete(path); else next.add(path);
    selectedPaths = next;
  }

  function createFromSelected() {
    if (!newSolutionName.trim() || selectedPaths.size === 0) return;
    const repos: SolutionRepo[] = [];
    for (const path of selectedPaths) {
      const scanned = scannedRepos.find(r => r.path === path);
      if (!scanned) continue;
      repos.push({
        repoId: scanned.repoId ?? path.replace(/^\//, ''),
        path,
        role: inferRepoRole(scanned),
        label: scanned.name,
      });
    }
    createSolution(newSolutionName.trim(), repos);
    selectedPaths = new Set();
    newSolutionName = '';
    showCreate = false;
  }

  async function scanFolder() {
    if (!scanRoot.trim()) return;
    // Fire and forget — daemon scans, registers, queues in background
    senseiApi(getPort()).scanFolder(scanRoot);
    scanRoot = '';
  }

  const STATUS_CLS: Record<string, string> = {
    active:    'bg-success-z2 text-success-z7',
    recent:    'bg-primary-z2 text-primary-z7',
    stale:     'bg-warning-z2 text-warning-z7',
    archived:  'bg-surface-z3 text-surface-z5',
    abandoned: 'bg-error-z2 text-error-z7',
    unknown:   'bg-surface-z3 text-surface-z5',
  };

  async function loadProjects() {
    const api = senseiApi(getPort());
    projects = await api.getProjects();
    scannedRepos = projects.map(p => ({
      name: p.name,
      path: p.path,
      repoId: p.repo_id,
      categories: [],
      status: (p.indexed_at ? 'active' : 'unknown') as any,
      tech_stack: p.stack ?? [],
      commit_count: 0,
    }));
  }

  let refreshInterval: ReturnType<typeof setInterval>;

  onMount(async () => {
    connectSSE(getPort());
    await loadProjects();
    refreshStatus();

    // Refresh project list every 3s — always poll while page is open
    // (queue status can briefly show null between jobs)
    refreshInterval = setInterval(async () => {
      await loadProjects();
      refreshStatus();
    }, 3000);
  });
</script>

<div class="flex h-full flex-col min-h-0">
  <div class="border-b border-surface-z0/50 px-4 py-2 shrink-0 flex items-center gap-3">
    <h1 class="text-sm font-semibold text-surface-z8">Projects</h1>
    <span class="text-xs text-surface-z4">{indexedCount}/{totalCount} indexed</span>
    {#if queueStatus.current}
      <span class="rounded px-1.5 py-0.5 text-[10px] bg-info-z2 text-info-z6">
        indexing: {queueStatus.current.repo_id}
      </span>
    {/if}
    {#if queueStatus.queued.length > 0}
      <span class="text-[10px] text-surface-z4">{queueStatus.queued.length} queued</span>
    {/if}
    {#if indexedCount > 0 && indexedCount < totalCount}
      <div class="w-20 h-1.5 rounded-full bg-surface-z3 overflow-hidden">
        <div class="h-full rounded-full bg-primary-z5 transition-all" style="width: {(indexedCount / totalCount * 100).toFixed(0)}%"></div>
      </div>
    {/if}
    <div class="ml-auto flex items-center gap-2">
      <button
        onclick={async () => {
          const api = senseiApi(getPort());
          for (const p of projects.filter(p => !p.indexed_at)) {
            await api.indexRepo(p.repo_id, p.path);
          }
          refreshStatus();
        }}
        class="rounded-md bg-primary-z2 px-2 py-1 text-[10px] font-medium text-primary-z7 hover:bg-primary-z3"
      >
        Index All
      </button>
      <input
        type="text"
        bind:value={search}
        placeholder="Search…"
        class="rounded-md border border-surface-z3 bg-surface-z1 px-2 py-1 text-xs text-surface-z7 outline-none focus:border-primary-z4 w-40"
      />
    </div>
  </div>

  <div class="flex-1 overflow-y-auto px-4 py-3 space-y-3">

    <!-- Scan folder -->
    <div class="flex items-center gap-2">
      <input
        type="text"
        bind:value={scanRoot}
        placeholder="~/Developer"
        class="flex-1 rounded-md border border-surface-z3 bg-surface-z1 px-2 py-1.5 text-xs text-surface-z7 outline-none focus:border-primary-z4"
      />
      <button
        onclick={scanFolder}
        disabled={scanning}
        class="rounded-md bg-primary-z2 px-3 py-1.5 text-xs font-medium text-primary-z7 hover:bg-primary-z3 transition-colors disabled:opacity-50"
      >
        {scanning ? 'Scanning…' : 'Scan'}
      </button>
    </div>

    <!-- Solution suggestions -->
    {#if suggestions.length > 0}
      <div class="space-y-1.5">
        <p class="text-[10px] font-semibold text-surface-z4 uppercase tracking-wide">Suggested solutions</p>
        {#each suggestions as suggestion}
          <div class="flex items-center gap-3 rounded-lg bg-primary-z1 border border-primary-z2 px-3 py-2">
            <span class="text-xs i-solar-lightbulb-bold-duotone text-primary-z5"></span>
            <div class="flex-1 min-w-0">
              <p class="text-xs font-medium text-primary-z7">Group as "{suggestion.name}"</p>
              <p class="text-[10px] text-primary-z4 truncate">
                {suggestion.paths.map(p => scannedRepos.find(r => r.path === p)?.name ?? p.split('/').at(-1)).join(', ')}
              </p>
            </div>
            <button
              onclick={() => acceptSuggestion(suggestion)}
              class="rounded bg-primary-z2 px-2 py-1 text-[10px] font-medium text-primary-z7 hover:bg-primary-z3 transition-colors shrink-0"
            >
              Create
            </button>
            <button
              onclick={() => dismissSuggestionHandler(suggestion)}
              class="text-[10px] text-primary-z3 hover:text-primary-z5 shrink-0"
            >
              Dismiss
            </button>
          </div>
        {/each}
      </div>
    {/if}

    <!-- Create solution from selection -->
    {#if selectedPaths.size > 0}
      <div class="flex items-center gap-2 rounded-lg bg-primary-z1 px-3 py-2">
        <span class="text-xs text-primary-z6">{selectedPaths.size} selected</span>
        {#if showCreate}
          <input
            type="text"
            bind:value={newSolutionName}
            placeholder="Solution name"
            class="flex-1 rounded border border-primary-z3 bg-white px-2 py-1 text-xs outline-none"
          />
          <button onclick={createFromSelected} class="rounded bg-primary-z5 px-2 py-1 text-xs text-white">Create</button>
          <button onclick={() => { showCreate = false; selectedPaths = new Set(); }} class="text-xs text-primary-z4">Cancel</button>
        {:else}
          <button onclick={() => showCreate = true} class="rounded bg-primary-z2 px-2 py-1 text-xs text-primary-z7">Create solution</button>
          <button onclick={() => selectedPaths = new Set()} class="text-xs text-primary-z4">Clear</button>
        {/if}
      </div>
    {/if}

    <!-- Repo list with live indexing status -->
    {#each filtered as repo (repo.path)}
      {@const assignedTo = getSolutionForPath(repo.path)}
      {@const proj = projects.find(p => p.path === repo.path)}
      {@const repoId = proj?.repo_id ?? repo.repoId ?? repo.name}
      {@const indexingNow = isIndexing(repoId)}
      {@const progress = getProgressForRepo(repoId)}
      <div class="flex items-center gap-3 rounded-lg bg-surface-z2/50 px-3 py-2 hover:bg-surface-z2 transition-colors">
        <input
          type="checkbox"
          checked={selectedPaths.has(repo.path)}
          onchange={() => toggleSelect(repo.path)}
          class="shrink-0"
        />

        <!-- Name + path (clickable) -->
        {#if assignedTo}
          <a href="/s/{assignedTo.id}" class="flex-1 min-w-0">
            <p class="text-sm text-surface-z7 truncate hover:text-primary-z6">{repo.name}</p>
            <p class="text-[10px] text-surface-z3 truncate">{repo.path}</p>
          </a>
        {:else}
          <a href="/p/{repoId}" class="flex-1 min-w-0">
            <p class="text-sm text-surface-z7 truncate hover:text-primary-z6">{repo.name}</p>
            <p class="text-[10px] text-surface-z3 truncate">{repo.path}</p>
          </a>
        {/if}

        <!-- Index status -->
        {#if indexingNow && progress}
          <span class="text-[10px] text-info-z6 shrink-0 max-w-48 truncate">
            {#if progress.files_processed != null && progress.files_total}
              {progress.files_processed}/{progress.files_total}
            {/if}
            {progress.current_file ? progress.current_file.split('/').at(-1) : 'starting...'}
          </span>
        {:else if indexingNow}
          <span class="text-[10px] text-info-z6 shrink-0">queued...</span>
        {:else if proj?.indexed_at}
          <span class="text-[10px] text-success-z5 shrink-0">indexed</span>
        {:else if proj?.last_error}
          <button
            onclick={() => startIndex(repoId, repo.path, true)}
            class="text-[10px] text-error-z5 shrink-0 hover:text-error-z6"
          >
            failed — retry
          </button>
        {:else}
          <button
            onclick={() => startIndex(repoId, repo.path)}
            class="text-[10px] text-primary-z5 shrink-0 hover:text-primary-z6"
          >
            index
          </button>
        {/if}

        <!-- Solution badge -->
        {#if assignedTo}
          <a href="/s/{assignedTo.id}" class="rounded bg-primary-z1 px-1.5 py-0.5 text-[10px] text-primary-z6 shrink-0 truncate max-w-24 hover:bg-primary-z2">{assignedTo.name}</a>
        {/if}
      </div>
    {/each}

    {#if filtered.length === 0}
      <div class="text-center py-12">
        <p class="text-sm text-surface-z4">No repos found.</p>
        <p class="text-xs text-surface-z3 mt-1">Scan a folder to discover repositories.</p>
      </div>
    {/if}

  </div>
</div>
