<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import type { WizardState, WizUpdate } from '../types.js';
  import { getPort } from '$lib/appstate.svelte.js';
  import { getRepoStore } from '$lib/repos.svelte.js';

  let { wizState, update, onScan }: {
    wizState: WizardState;
    update: WizUpdate;
    onScan?: () => Promise<void>;
  } = $props();

  // Use the shared RepoStore — it handles SSE properly
  const store = getRepoStore(getPort());

  // Local state
  let scanning = $state(false);
  let done = $state(false);

  // Derived from the store's reactive state
  const roots = $derived(wizState.folders.length);
  const repoCount = $derived(store.totalCount);
  const indexedCount = $derived(store.indexedCount);
  const indexingCount = $derived(store.indexingCount);
  const totalFiles = $derived(store.totalFiles);
  const completedFiles = $derived(store.completedFiles);
  const anyIndexing = $derived(store.anyIndexing);
  const repos = $derived(store.all);

  // Watch for indexing completion
  $effect(() => {
    if (scanning && !anyIndexing && repoCount > 0 && !done) {
      done = true;
      scanning = false;
      update({ scanDone: true });
      if (onScan) onScan();
    }
  });

  onMount(() => {
    store.connect();
  });

  onDestroy(() => {
    store.disconnect();
  });

  async function startScan() {
    scanning = true;
    done = false;
    update({ scanStarted: true });

    // Trigger scan for each folder via the store
    for (const folder of wizState.folders) {
      await store.scanFolder(folder.path);
    }
  }
</script>

<section class="step">
  <div class="step-label"><span class="kanji">五</span> STEP</div>
  <h1 class="display headline">
    {#if done}Scan complete{:else}Scan{/if}
  </h1>
  <p class="subtitle">
    {#if !scanning && !done}
      Ready to scan {roots} root{roots !== 1 ? 's' : ''}.
    {:else if done}
      The map is drawn.
    {:else}
      Scanning · {completedFiles} / {totalFiles} files
    {/if}
  </p>

  {#if !scanning && !done}
    <div class="hero-card">
      <div class="hero-kanji kanji">探</div>
      <p class="hero-text">
        The daemon will recurse your folders, identify repositories,
        and extract the code graph.
      </p>
      <button class="begin-btn" onclick={startScan}>Begin scan →</button>
    </div>
  {:else}
    <!-- Stats bar -->
    <div class="stats-bar">
      <div class="stat"><div class="stat-value">{roots}</div><div class="stat-label">ROOTS</div></div>
      <div class="stat"><div class="stat-value">{repoCount}</div><div class="stat-label">REPOS</div></div>
      <div class="stat"><div class="stat-value">{totalFiles}</div><div class="stat-label">FILES</div></div>
      <div class="stat"><div class="stat-value">{indexedCount}</div><div class="stat-label">INDEXED</div></div>
    </div>

    <!-- Live repo list from store -->
    <div class="repo-list">
      {#each repos as entry}
        <div class="repo-row" data-state={entry.indexState}>
          <div class="repo-info">
            <span class="repo-name">{entry.project.name}</span>
            <span class="repo-path">{entry.project.path}</span>
          </div>
          <div class="repo-progress">
            {#if entry.indexState === 'indexing' || entry.indexState === 'queued'}
              <div class="progress-bar">
                <div class="progress-fill"
                  style="width: {entry.filesTotal > 0 ? (entry.filesCompleted / entry.filesTotal * 100) : 0}%">
                </div>
              </div>
              <span class="repo-count">{entry.filesCompleted} / {entry.filesTotal}</span>
            {:else if entry.indexState === 'indexed'}
              <span class="repo-status done">indexed</span>
            {:else if entry.indexState === 'failed'}
              <span class="repo-status failed">failed</span>
            {:else}
              <span class="repo-status idle">idle</span>
            {/if}
          </div>
        </div>
      {/each}

      {#if repos.length === 0 && scanning}
        <div class="empty-hint">Discovering repos...</div>
      {/if}
    </div>
  {/if}
</section>

<style>
  .step { max-width: 780px; }
  .step-label { font-size: 12px; letter-spacing: 0.12em; color: var(--sumi-3); margin-bottom: var(--space-2); }
  .step-label .kanji { color: var(--shu); margin-right: 4px; }
  .headline { font-size: 40px; color: var(--sumi); margin: 0 0 var(--space-2) 0; line-height: 1.15; }
  .subtitle { font-size: 15px; color: var(--sumi-3); margin: 0 0 var(--space-8) 0; }

  .hero-card {
    background: var(--paper-2); border-radius: var(--radius-lg);
    padding: var(--space-12) var(--space-10);
    display: flex; flex-direction: column; align-items: center; text-align: center; gap: var(--space-6);
  }
  .hero-kanji { font-size: 80px; color: var(--shu); opacity: 0.25; line-height: 1; }
  .hero-text { font-size: 15px; color: var(--sumi-2); max-width: 440px; line-height: 1.6; margin: 0; }
  .begin-btn {
    font-size: 14px; background: var(--paper-3); color: var(--sumi);
    padding: 12px 28px; border-radius: var(--radius);
    border: var(--border-card); cursor: pointer; transition: opacity 0.14s;
  }
  .begin-btn:hover { opacity: 0.88; }

  .stats-bar {
    display: grid; grid-template-columns: repeat(4, 1fr); gap: var(--space-4);
    margin-bottom: var(--space-6); padding-bottom: var(--space-6); border-bottom: var(--hairline);
  }
  .stat { text-align: center; }
  .stat-value { font-family: var(--font-display); font-size: 28px; color: var(--sumi); line-height: 1.2; }
  .stat-label { font-size: 10px; letter-spacing: 0.12em; color: var(--sumi-3); margin-top: var(--space-1); }

  .repo-list { display: flex; flex-direction: column; gap: var(--space-2); }
  .repo-row {
    display: flex; align-items: center; gap: var(--space-4);
    padding: var(--space-3) var(--space-4);
    background: var(--paper-2); border-radius: var(--radius);
  }
  .repo-row[data-state="indexing"] { border-left: 2px solid var(--amber); }
  .repo-row[data-state="indexed"] { border-left: 2px solid var(--jade); }
  .repo-row[data-state="failed"] { border-left: 2px solid var(--shu); }
  .repo-info { flex: 1; min-width: 0; }
  .repo-name { font-size: 13px; font-weight: 500; color: var(--sumi); display: block; }
  .repo-path { font-size: 11px; color: var(--sumi-4); font-family: var(--font-mono); display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .repo-progress { display: flex; align-items: center; gap: var(--space-2); flex-shrink: 0; }
  .progress-bar { width: 80px; height: 4px; background: var(--paper-3); border-radius: 2px; overflow: hidden; }
  .progress-fill { height: 100%; background: var(--amber); border-radius: 2px; transition: width 0.3s; }
  .repo-count { font-size: 11px; color: var(--sumi-3); font-family: var(--font-mono); min-width: 60px; text-align: right; }
  .repo-status { font-size: 11px; font-weight: 500; }
  .repo-status.done { color: var(--jade); }
  .repo-status.failed { color: var(--shu); }
  .repo-status.idle { color: var(--sumi-4); }
  .empty-hint { font-size: 13px; color: var(--sumi-4); padding: var(--space-6); text-align: center; }
</style>
