<script lang="ts">
  import { wizardState } from '$lib/wizard-state.svelte.js';

  const assistants = $derived(wizardState.assistants.assistants);

  function toggle(id: string) {
    const asst = assistants.find(a => a.id === id);
    if (asst) asst.selected = !asst.selected;
  }

  function shortPath(p: string): string {
    return p.replace(/^\/Users\/[^/]+/, '~');
  }
</script>

<div class="assistants">
  <p class="step-desc">
    Registers plugins, skills, commands, agents, logging and metrics.
  </p>

  <div class="grid">
    {#each assistants as asst (asst.id)}
      <button
        class="card"
        class:card-found={asst.installed}
        class:card-missing={!asst.installed}
        onclick={() => toggle(asst.id)}
      >
        <div class="card-body">
          <span class="card-name">{asst.name}</span>
          {#if asst.installed && asst.config_path}
            <span class="card-path">{shortPath(asst.config_path)}</span>
          {:else}
            <span class="card-notfound">not found</span>
          {/if}
        </div>
        <div class="checkbox" class:checked={asst.selected}>
          {#if asst.selected}<span class="check">&#10003;</span>{/if}
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
    background: var(--paper); border: var(--border-card);
    transition: border-color 0.14s, opacity 0.14s;
    font-family: var(--font-ui);
  }
  .card-found { border-color: var(--sumi-4); }
  .card-missing { opacity: 0.55; }
  .card-missing:hover { opacity: 0.7; }

  .card-body { flex: 1; min-width: 0; }
  .card-name { font-size: 15px; font-weight: 600; display: block; }
  .card-path {
    font-size: 12px; color: var(--sumi-3); font-family: var(--font-mono);
    display: block; margin-top: 4px;
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
  }
  .card-notfound { font-size: 12px; color: var(--sumi-4); font-style: italic; display: block; margin-top: 4px; }

  .checkbox {
    width: 22px; height: 22px; border-radius: 4px;
    border: 2px solid var(--paper-edge);
    display: flex; align-items: center; justify-content: center;
    flex-shrink: 0; background: var(--paper); transition: all 0.14s;
  }
  .checkbox.checked { border-color: var(--sumi-4); background: var(--sumi); }
  .check { color: var(--paper); font-size: 12px; font-weight: 700; }

  .empty { font-size: 13px; color: var(--sumi-3); font-style: italic; }
</style>
