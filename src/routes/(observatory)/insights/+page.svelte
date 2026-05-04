<script lang="ts">
  import { appState } from '$lib/appstate.svelte.js';

  type Memory = {
    id: string;
    title: string;
    content: string;
    scope: string;
    strength: number;
    status: string;
    type: string;
    project_name?: string;
  };

  let memories = $state<Memory[]>([]);
  let loading = $state(false);
  let tab = $state<'all' | 'memories' | 'patterns' | 'corrections'>('all');
  let selectedMemory = $state<Memory | null>(null);

  function strengthDots(s: number): string {
    const filled = Math.round(s);
    return '●'.repeat(filled) + '○'.repeat(5 - filled);
  }
</script>

<div class="page">
  <header class="page-header">
    <p class="date-label">Insights</p>
    <h1 class="display page-title">學 Insights</h1>
  </header>

  <!-- Tabs -->
  <div class="tab-row">
    {#each [['all', 'All'], ['memories', 'Memories'], ['patterns', 'Patterns'], ['corrections', 'Corrections']] as [key, label]}
      <button class="tab" class:active={tab === key} onclick={() => tab = key as any}>{label}</button>
    {/each}
  </div>

  {#if loading}
    <p class="hint">Loading...</p>
  {:else if memories.length === 0}
    <div class="empty-state">
      <span class="kanji empty-kanji">學</span>
      <p class="display empty-title">Nothing learned yet.</p>
      <p class="empty-body">
        As you work with your assistants, sensei observes corrections and patterns.
        Learnings appear here once sensei has enough evidence to teach.
      </p>
    </div>
  {:else}
    <div class="content-grid">
      <!-- Memory list -->
      <div class="memory-list">
        {#each memories as mem (mem.id)}
          <button
            class="memory-card"
            class:selected={selectedMemory?.id === mem.id}
            onclick={() => selectedMemory = mem}
          >
            <div class="memory-header">
              <span class="memory-strength" title="Strength {mem.strength}/5">{strengthDots(mem.strength)}</span>
              <span class="memory-scope">{mem.scope}</span>
            </div>
            <p class="memory-title">{mem.title}</p>
            {#if mem.project_name}
              <span class="memory-project">{mem.project_name}</span>
            {/if}
          </button>
        {/each}
      </div>

      <!-- Detail drawer -->
      {#if selectedMemory}
        <div class="memory-drawer">
          <div class="drawer-section">
            <p class="drawer-label">What</p>
            <p class="drawer-text">{selectedMemory.title}</p>
          </div>
          <div class="drawer-section">
            <p class="drawer-label">Why</p>
            <p class="drawer-text">{selectedMemory.content}</p>
          </div>
          <div class="drawer-section">
            <p class="drawer-label">Scope</p>
            <span class="scope-tag">{selectedMemory.scope}</span>
          </div>
          <div class="drawer-section">
            <p class="drawer-label">Strength</p>
            <span class="strength-display">{strengthDots(selectedMemory.strength)} {selectedMemory.strength}/5</span>
          </div>
          <div class="drawer-section">
            <p class="drawer-label">Status</p>
            <span class="status-tag">{selectedMemory.status}</span>
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

  /* ── Tabs ────────────────────────────────────────────────── */
  .tab-row {
    display: flex;
    gap: 0;
    border-bottom: var(--hairline);
    margin-bottom: 28px;
  }
  .tab {
    padding: 8px 18px;
    border: none;
    background: none;
    color: var(--sumi-3);
    font-size: 13px;
    cursor: pointer;
    border-bottom: 2px solid transparent;
    margin-bottom: -1px;
  }
  .tab:hover { color: var(--sumi-2); }
  .tab.active { color: var(--sumi); border-bottom-color: var(--shu); }

  /* ── Empty state ────────────────────────────────────────── */
  .empty-state { text-align: center; padding: 80px 20px; }
  .empty-kanji { font-size: 64px; color: var(--shu); opacity: 0.3; }
  .empty-title { font-size: 20px; font-weight: 400; margin: 16px 0 8px; }
  .empty-body { font-size: 13px; color: var(--sumi-3); max-width: 380px; margin: 0 auto; line-height: 1.65; }

  /* ── Content grid ───────────────────────────────────────── */
  .content-grid {
    display: grid;
    grid-template-columns: 1fr 340px;
    gap: 24px;
  }
  .memory-list { display: flex; flex-direction: column; gap: 4px; }
  .memory-card {
    text-align: left;
    padding: 14px 16px;
    border: var(--border-card);
    border-radius: var(--radius);
    background: var(--paper);
    cursor: pointer;
    transition: background 0.1s;
  }
  .memory-card:hover { background: var(--paper-2); }
  .memory-card.selected { border-color: var(--sumi-3); background: var(--paper-2); }
  .memory-header { display: flex; justify-content: space-between; margin-bottom: 6px; }
  .memory-strength { font-size: 10px; color: var(--shu); letter-spacing: 1px; }
  .memory-scope { font-size: 10px; color: var(--sumi-4); text-transform: uppercase; letter-spacing: 0.1em; }
  .memory-title { font-size: 13px; color: var(--sumi); margin: 0; line-height: 1.5; }
  .memory-project { font-size: 11px; color: var(--sumi-3); margin-top: 4px; display: inline-block; }

  /* ── Drawer ──────────────────────────────────────────────── */
  .memory-drawer {
    padding: 24px;
    background: var(--paper-2);
    border: var(--border-card);
    border-radius: var(--radius-lg);
    position: sticky;
    top: 24px;
  }
  .drawer-section { margin-bottom: 20px; }
  .drawer-section:last-child { margin-bottom: 0; }
  .drawer-label {
    font-size: 9.5px;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: var(--sumi-3);
    margin: 0 0 6px;
  }
  .drawer-text { font-size: 13px; color: var(--sumi); margin: 0; line-height: 1.6; }
  .scope-tag, .status-tag {
    display: inline-block;
    padding: 3px 10px;
    border-radius: 100px;
    font-size: 11px;
    background: var(--paper-3);
    color: var(--sumi-2);
  }
  .strength-display { font-size: 13px; color: var(--shu); }
</style>
