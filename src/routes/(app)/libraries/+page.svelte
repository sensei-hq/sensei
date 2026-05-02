<script lang="ts">
  import { onMount } from 'svelte';
  import { appState } from '$lib/appstate.svelte.js';
  import { senseiApi } from '$lib/api.js';
  import type { LibEntry } from '$lib/types.js';

  let libs = $state<LibEntry[]>([]);
  let loading = $state(true);
  let search = $state('');
  let kindFilter = $state<'all' | 'code' | 'service'>('all');
  let selectedLib = $state<LibEntry | null>(null);

  onMount(async () => {
    await appState.load();
    const api = senseiApi(appState.port);
    const data = await api.getLibs({ shared: true });
    libs = data.libs;
    loading = false;
  });

  let filtered = $derived(
    libs.filter(l => {
      if (search && !l.name.toLowerCase().includes(search.toLowerCase())) return false;
      return true;
    })
  );
</script>

<div class="page">
  <header class="page-header">
    <p class="date-label">Libraries</p>
    <h1 class="display page-title">書 Libraries</h1>
  </header>

  <!-- Search + filters -->
  <div class="toolbar">
    <input
      class="search-input"
      type="text"
      placeholder="Search libraries..."
      bind:value={search}
    />
    <div class="filter-row">
      {#each [['all', 'All'], ['code', 'Code'], ['service', 'Services']] as [key, label]}
        <button class="filter-chip" class:active={kindFilter === key} onclick={() => kindFilter = key as any}>{label}</button>
      {/each}
    </div>
  </div>

  {#if loading}
    <p class="hint">Loading libraries...</p>
  {:else if filtered.length === 0}
    <div class="empty-state">
      <span class="kanji empty-kanji">書</span>
      <p class="display empty-title">No libraries indexed.</p>
      <p class="empty-body">
        Libraries appear once sensei scans your project dependencies.
        Add folders in the setup wizard, and sensei will detect libraries from your manifests.
      </p>
    </div>
  {:else}
    <div class="content-grid">
      <div class="lib-list">
        {#each filtered as lib (lib.name)}
          <button
            class="lib-card"
            class:selected={selectedLib?.name === lib.name}
            onclick={() => selectedLib = lib}
          >
            <div class="lib-header">
              <span class="lib-name">{lib.name}</span>
              <span class="lib-count">{lib.repoCount} repo{lib.repoCount !== 1 ? 's' : ''}</span>
            </div>
          </button>
        {/each}
      </div>

      {#if selectedLib}
        <div class="lib-detail">
          <h3 class="detail-name">{selectedLib.name}</h3>
          <div class="detail-section">
            <p class="detail-label">Used in</p>
            <div class="repo-tags">
              {#each selectedLib.repos as repo}
                <span class="lib-tag">{repo}</span>
              {/each}
            </div>
          </div>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .page {
    max-width: 960px;
    margin: 0 auto;
    padding: 48px 48px 64px;
  }
  .page-header { margin-bottom: 24px; }
  .date-label {
    font-size: 10.5px;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: var(--sumi-3);
    margin: 0 0 8px;
  }
  .page-title { font-size: 24px; font-weight: 400; margin: 0; }

  /* ── Toolbar ────────────────────────────────────────────── */
  .toolbar {
    display: flex;
    align-items: center;
    gap: 16px;
    margin-bottom: 24px;
  }
  .search-input {
    flex: 1;
    padding: 8px 14px;
    border: var(--border-input);
    border-radius: var(--radius);
    background: var(--paper);
    color: var(--sumi);
    font-size: 13px;
    font-family: var(--font-ui);
    outline: none;
  }
  .search-input:focus { border: var(--border-focus); }
  .filter-row { display: flex; gap: 6px; }
  .filter-chip {
    padding: 5px 14px;
    border-radius: 100px;
    border: var(--border-card);
    background: transparent;
    color: var(--sumi-2);
    font-size: 12px;
    cursor: pointer;
  }
  .filter-chip:hover { background: var(--paper-2); }
  .filter-chip.active { background: var(--sumi); color: var(--paper); border-color: var(--sumi); }

  /* ── Empty state ────────────────────────────────────────── */
  .empty-state { text-align: center; padding: 80px 20px; }
  .empty-kanji { font-size: 64px; color: var(--shu); opacity: 0.3; }
  .empty-title { font-size: 20px; font-weight: 400; margin: 16px 0 8px; }
  .empty-body { font-size: 13px; color: var(--sumi-3); max-width: 380px; margin: 0 auto; line-height: 1.65; }

  /* ── Content grid ───────────────────────────────────────── */
  .content-grid { display: grid; grid-template-columns: 1fr 340px; gap: 24px; }
  .lib-list { display: flex; flex-direction: column; gap: 4px; }
  .lib-card {
    text-align: left;
    padding: 14px 16px;
    border: var(--border-card);
    border-radius: var(--radius);
    background: var(--paper);
    cursor: pointer;
    transition: background 0.1s;
  }
  .lib-card:hover { background: var(--paper-2); }
  .lib-card.selected { border-color: var(--sumi-3); background: var(--paper-2); }
  .lib-header { display: flex; align-items: baseline; gap: 8px; margin-bottom: 6px; }
  .lib-name { font-size: 13px; font-weight: 500; color: var(--sumi); }
  .lib-count { font-size: 11px; color: var(--sumi-3); }
  .lib-tag {
    display: inline-block;
    padding: 2px 8px;
    border-radius: 100px;
    font-size: 10px;
    background: var(--paper-3);
    color: var(--sumi-3);
    text-transform: lowercase;
  }

  /* ── Detail panel ───────────────────────────────────────── */
  .lib-detail {
    padding: 24px;
    background: var(--paper-2);
    border: var(--border-card);
    border-radius: var(--radius-lg);
    position: sticky;
    top: 24px;
  }
  .detail-name { font-size: 16px; margin: 0 0 16px; }
  .detail-section { margin-bottom: 16px; }
  .repo-tags { display: flex; flex-wrap: wrap; gap: 6px; }
  .detail-label {
    font-size: 9.5px;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: var(--sumi-3);
    margin: 0 0 6px;
  }
</style>
