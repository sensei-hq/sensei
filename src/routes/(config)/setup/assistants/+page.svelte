<script lang="ts">
  import { wizardState } from '$lib/wizard-state.svelte.js';

  const assistants = $derived(wizardState.assistants.assistants);

  // What gets registered when each family is enabled.
  // Claude gets the full plugin suite; all others get MCP server only.
  const CAPABILITIES: Record<string, string[]> = {
    claude: ['plugins', 'skills', 'commands', 'agents'],
  };

  function caps(id: string): string[] {
    return CAPABILITIES[id] ?? ['mcp server'];
  }

  function toggle(id: string) {
    const fam = assistants.find(a => a.id === id);
    if (fam && fam.variants.some(v => v.installed)) fam.selected = !fam.selected;
  }
</script>

<div class="assistants">
  <p class="step-desc">
    Registers plugins, skills, commands, agents, logging and metrics.
  </p>

  <div class="grid">
    {#each assistants as fam (fam.id)}
      {@const installedCount = fam.variants.filter(v => v.installed).length}
      {@const anyInstalled = installedCount > 0}
      <button
        class="card"
        class:card-missing={!anyInstalled}
        class:card-selected={fam.selected}
        onclick={() => toggle(fam.id)}
      >
        <div class="card-body">
          <div class="card-title-row">
            <span class="card-name">{fam.name}</span>
            {#if anyInstalled}
              <span class="variant-count">{installedCount} detected</span>
            {:else}
              <span class="chip-notfound">not found</span>
            {/if}
          </div>
          <div class="chips">
            {#each caps(fam.id) as cap}
              <span class="chip">{cap}</span>
            {/each}
          </div>
        </div>
        <div class="checkbox" class:checked={fam.selected}>
          {#if fam.selected}<span class="check">&#10003;</span>{/if}
        </div>
      </button>
    {/each}
  </div>

  {#if assistants.length === 0}
    <p class="empty">No AI coding assistants detected. Make sure the daemon is running.</p>
  {/if}
</div>

<style>
  .step-desc {
    font-size: 14px; color: var(--sumi-3); line-height: 1.6;
    margin: 0 0 24px;
  }

  .grid {
    display: grid; grid-template-columns: 1fr 1fr; gap: var(--space-3);
  }

  .card {
    display: flex; align-items: center; gap: var(--space-4);
    padding: var(--space-5) var(--space-6);
    border-radius: var(--radius-lg); cursor: pointer; text-align: left;
    background: var(--paper); border: 1px solid var(--paper-3);
    transition: border-color 0.14s, background 0.14s, opacity 0.14s;
    font-family: var(--font-ui);
    min-width: 0;
  }
  .card-selected { border: 1.5px solid var(--sumi-3); background: var(--paper-2); }
  .card-missing { opacity: 0.55; cursor: default; }
  .card-missing:hover { opacity: 0.7; }

  .card-body { flex: 1; min-width: 0; }
  .card-title-row {
    display: flex; align-items: baseline; justify-content: space-between;
    gap: 8px; margin-bottom: 6px;
  }
  .card-name { font-size: 15px; font-weight: 600; }

  .chips {
    display: flex; flex-wrap: wrap; gap: 5px;
  }

  .chip {
    font-size: 11px; font-family: var(--font-mono);
    color: var(--sumi-2);
    padding: 2px 8px;
    background: var(--paper-3);
    border-radius: 3px;
    white-space: nowrap;
  }
  .card-selected .chip {
    background: var(--paper);
    color: var(--sumi-2);
  }

  .variant-count {
    font-size: 11px; color: var(--sumi-4); white-space: nowrap;
  }

  .chip-notfound {
    font-size: 11px; color: var(--sumi-4); font-style: italic;
  }

  .checkbox {
    width: 22px; height: 22px; border-radius: 4px;
    border: 2px solid var(--paper-edge);
    display: flex; align-items: center; justify-content: center;
    flex-shrink: 0; background: var(--paper); transition: all 0.14s;
  }
  .checkbox.checked { border-color: var(--sumi-4); background: var(--sumi-3); }
  .check { color: var(--paper); font-size: 12px; font-weight: 700; }

  .empty { font-size: 13px; color: var(--sumi-3); font-style: italic; }
</style>
