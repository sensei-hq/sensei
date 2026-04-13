<script lang="ts">
  import { onMount } from 'svelte';
  import { getSolutions } from '$lib/solutions.svelte.js';
  import { senseiApi } from '$lib/api.js';
  import type { ServerProject, IndexProgress, SolutionRepo } from '$lib/types.js';

  let port = $state(parseInt(localStorage.getItem('sensei:port') ?? '7744', 10));

  let serverProjects = $state<ServerProject[]>([]);
  let progressMap = $state<Record<string, IndexProgress>>({});
  let activeQueue = $state<string[]>([]);
  let loading = $state(true);

  // Per-repo indexing state
  let indexingRepos = $state<Set<string>>(new Set());
  let errors = $state<Record<string, string>>({});

  // All repos across all solutions
  interface RepoRow {
    repoId: string;
    path: string;
    label: string;
    solutionName: string;
  }

  let allRepos = $derived((): RepoRow[] => {
    const rows: RepoRow[] = [];
    for (const s of getSolutions()) {
      for (const r of s.repos) {
        rows.push({
          repoId: r.repoId,
          path: r.path,
          label: r.label ?? r.path.split('/').at(-1) ?? r.repoId,
          solutionName: s.name,
        });
      }
    }
    return rows;
  });

  // Stats
  let totalCount = $derived(allRepos().length);
  let indexedCount = $derived(serverProjects.filter(p => allRepoIds().has(p.repoId) && p.indexedAt).length);
  let failedCount = $derived(serverProjects.filter(p => allRepoIds().has(p.repoId) && p.lastError).length);
  let partialCount = $derived(serverProjects.filter(p => allRepoIds().has(p.repoId) && p.partiallyIndexed).length);
  let runningCount = $derived(activeQueue.filter(id => allRepoIds().has(id)).length);

  function allRepoIds(): Set<string> {
    return new Set(allRepos().map(r => r.repoId));
  }

  function getStatus(repoId: string): 'indexed' | 'indexing' | 'failed' | 'partial' | 'pending' {
    if (indexingRepos.has(repoId) || activeQueue.includes(repoId)) return 'indexing';
    const p = serverProjects.find(p => p.repoId === repoId);
    if (p?.indexedAt) return 'indexed';
    if (p?.lastError) return 'failed';
    if (p?.partiallyIndexed) return 'partial';
    return 'pending';
  }

  const STATUS_CLS: Record<string, string> = {
    indexed: 'bg-success-z2 text-success-z7',
    indexing: 'bg-primary-z2 text-primary-z7',
    failed: 'bg-error-z2 text-error-z7',
    partial: 'bg-warning-z2 text-warning-z7',
    pending: 'bg-surface-z3 text-surface-z5',
  };

  async function load() {
    try {
      const api = senseiApi(port);
      const [projects, health] = await Promise.all([
        api.getProjects(),
        api.getHealth(),
      ]);
      serverProjects = projects;
      progressMap = (health as any).progress ?? {};
      activeQueue = (health as any).indexing ?? [];
    } catch { /* ignore */ }
    loading = false;
  }

  async function indexRepo(repoId: string, path: string, force = false) {
    const next = new Set(indexingRepos);
    next.add(repoId);
    indexingRepos = next;
    delete errors[repoId];
    errors = { ...errors };

    try {
      const api = senseiApi(port);
      const name = allRepos().find(r => r.repoId === repoId)?.label ?? repoId;
      await api.registerProject(repoId, name, path);
      const res = await api.indexRepo(repoId, path, force);
      if (!res.ok) {
        errors = { ...errors, [repoId]: 'Failed to queue index' };
      }
    } catch (e) {
      errors = { ...errors, [repoId]: String(e) };
    } finally {
      const next = new Set(indexingRepos);
      next.delete(repoId);
      indexingRepos = next;
      await load();
    }
  }

  async function indexAll(force = false) {
    for (const repo of allRepos()) {
      const status = getStatus(repo.repoId);
      if (!force && status === 'indexed') continue;
      indexRepo(repo.repoId, repo.path, force);
    }
  }

  async function retryAllFailed() {
    for (const repo of allRepos()) {
      const status = getStatus(repo.repoId);
      if (status === 'failed' || status === 'partial') {
        indexRepo(repo.repoId, repo.path, true);
      }
    }
  }

  onMount(() => {
    load();
    const interval = setInterval(load, 5000);
    return () => clearInterval(interval);
  });
</script>

<div class="flex h-full flex-col min-h-0">
  <div class="border-b border-surface-z0/50 px-4 py-2 shrink-0">
    <h1 class="text-sm font-semibold text-surface-z8">Indexer</h1>
  </div>

  <div class="flex-1 overflow-y-auto px-5 py-4 space-y-4">

    <!-- Stats bar -->
    <div class="grid grid-cols-5 gap-3">
      <div class="rounded-lg bg-surface-z2 p-3 text-center">
        <p class="text-[10px] text-surface-z4 uppercase">Total</p>
        <p class="text-xl font-semibold text-surface-z8">{totalCount}</p>
      </div>
      <div class="rounded-lg bg-surface-z2 p-3 text-center">
        <p class="text-[10px] text-success-z5 uppercase">Indexed</p>
        <p class="text-xl font-semibold text-success-z6">{indexedCount}</p>
      </div>
      <div class="rounded-lg bg-surface-z2 p-3 text-center">
        <p class="text-[10px] text-primary-z5 uppercase">Running</p>
        <p class="text-xl font-semibold text-primary-z6">{runningCount}</p>
      </div>
      <div class="rounded-lg bg-surface-z2 p-3 text-center">
        <p class="text-[10px] text-error-z5 uppercase">Failed</p>
        <p class="text-xl font-semibold text-error-z6">{failedCount}</p>
      </div>
      <div class="rounded-lg bg-surface-z2 p-3 text-center">
        <p class="text-[10px] text-warning-z5 uppercase">Partial</p>
        <p class="text-xl font-semibold text-warning-z6">{partialCount}</p>
      </div>
    </div>

    <!-- Actions -->
    <div class="flex items-center gap-2">
      <button
        onclick={() => indexAll(false)}
        class="rounded-md bg-primary-z2 px-3 py-1.5 text-xs font-medium text-primary-z7 hover:bg-primary-z3 transition-colors"
      >
        Index all unindexed
      </button>
      {#if failedCount + partialCount > 0}
        <button
          onclick={retryAllFailed}
          class="rounded-md bg-warning-z2 px-3 py-1.5 text-xs font-medium text-warning-z7 hover:bg-warning-z3 transition-colors"
        >
          Retry all failed ({failedCount + partialCount})
        </button>
      {/if}
      <button
        onclick={() => indexAll(true)}
        class="rounded-md bg-surface-z3 px-3 py-1.5 text-xs font-medium text-surface-z6 hover:bg-surface-z4 transition-colors"
      >
        Force re-index all
      </button>
    </div>

    <!-- Currently indexing -->
    {#if runningCount > 0 || indexingRepos.size > 0}
      <div>
        <p class="text-[10px] font-semibold text-surface-z5 uppercase tracking-wide mb-2">Currently indexing</p>
        {#each allRepos().filter(r => getStatus(r.repoId) === 'indexing') as repo (repo.repoId)}
          {@const prog = progressMap[repo.repoId]}
          <div class="rounded-lg bg-primary-z1 border border-primary-z2 px-3 py-2.5 mb-1.5 space-y-1.5">
            <div class="flex items-center gap-2">
              <span class="i-solar-refresh-bold-duotone animate-spin text-xs text-primary-z5"></span>
              <span class="text-sm font-medium text-primary-z7 flex-1 truncate">{repo.label}</span>
              <span class="rounded bg-primary-z2 px-1.5 py-0.5 text-[9px] text-primary-z5">{repo.solutionName}</span>
              {#if prog}
                <span class="text-[10px] text-primary-z5 shrink-0">
                  {prog.filesProcessed}/{prog.filesTotal - (prog.filesUnchanged ?? 0)} files
                </span>
              {/if}
            </div>
            {#if prog}
              <div class="space-y-0.5">
                <div class="h-1.5 rounded-full bg-primary-z2 overflow-hidden">
                  <div class="h-full rounded-full bg-primary-z5 transition-all" style="width: {Math.min(Math.round((prog.filesProcessed / Math.max(1, prog.filesTotal - (prog.filesUnchanged ?? 0))) * 100), 100)}%"></div>
                </div>
                <p class="text-[10px] text-primary-z4 truncate">{prog.currentFile}</p>
              </div>
            {/if}
          </div>
        {/each}
      </div>
    {/if}

    <!-- All repos -->
    <div>
      <p class="text-[10px] font-semibold text-surface-z5 uppercase tracking-wide mb-2">All repos ({totalCount})</p>
      <div class="rounded-lg border border-surface-z0/50 divide-y divide-surface-z0/30">
        {#each allRepos() as repo (repo.repoId)}
          {@const status = getStatus(repo.repoId)}
          {@const serverInfo = serverProjects.find(p => p.repoId === repo.repoId)}
          {@const prog = progressMap[repo.repoId]}
          <div class="px-3 py-2 flex items-center gap-3">
            <span class="rounded px-1.5 py-0.5 text-[10px] font-medium {STATUS_CLS[status]}">{status}</span>
            <div class="flex-1 min-w-0">
              <div class="flex items-center gap-2">
                <p class="text-sm text-surface-z7 truncate">{repo.label}</p>
                <span class="rounded bg-surface-z3 px-1 py-0.5 text-[9px] text-surface-z5 shrink-0">{repo.solutionName}</span>
              </div>
              {#if status === 'indexing' && prog}
                <div class="flex items-center gap-2 mt-0.5">
                  <div class="flex-1 h-1 rounded-full bg-surface-z3 overflow-hidden">
                    <div class="h-full rounded-full bg-primary-z5 transition-all" style="width: {Math.min(Math.round((prog.filesProcessed / Math.max(1, prog.filesTotal - (prog.filesUnchanged ?? 0))) * 100), 100)}%"></div>
                  </div>
                  <span class="text-[10px] text-surface-z4 shrink-0">{prog.filesProcessed}/{prog.filesTotal - (prog.filesUnchanged ?? 0)}</span>
                </div>
                <p class="text-[10px] text-surface-z3 truncate mt-0.5">{prog.currentFile}</p>
              {:else if serverInfo?.indexedAt}
                <p class="text-[10px] text-surface-z3">{new Date(serverInfo.indexedAt).toLocaleString()}</p>
              {/if}
              {#if errors[repo.repoId]}
                <p class="text-[10px] text-error-z5 truncate mt-0.5">{errors[repo.repoId]}</p>
              {:else if serverInfo?.lastError}
                <p class="text-[10px] text-error-z5 truncate mt-0.5">{serverInfo.lastError}</p>
              {/if}
            </div>
            {#if status === 'failed' || status === 'partial'}
              <button
                onclick={() => indexRepo(repo.repoId, repo.path, true)}
                class="rounded px-2 py-1 text-[10px] font-medium bg-warning-z2 text-warning-z7 hover:bg-warning-z3 transition-colors shrink-0"
              >
                Retry
              </button>
            {:else if status === 'pending'}
              <button
                onclick={() => indexRepo(repo.repoId, repo.path, false)}
                class="rounded px-2 py-1 text-[10px] font-medium bg-primary-z2 text-primary-z7 hover:bg-primary-z3 transition-colors shrink-0"
              >
                Index
              </button>
            {:else if status === 'indexed'}
              <button
                onclick={() => indexRepo(repo.repoId, repo.path, false)}
                class="rounded px-2 py-1 text-[10px] font-medium bg-surface-z3 text-surface-z6 hover:bg-surface-z4 transition-colors shrink-0"
              >
                Re-index
              </button>
            {/if}
          </div>
        {/each}
      </div>
    </div>

    {#if totalCount === 0}
      <div class="text-center py-12">
        <p class="text-sm text-surface-z4">No repos found.</p>
        <p class="text-xs text-surface-z3 mt-1">Import repos via the setup wizard or All Repos page.</p>
      </div>
    {/if}

  </div>
</div>
