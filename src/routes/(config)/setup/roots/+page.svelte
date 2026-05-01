<script lang="ts">
  import { wizardState } from '$lib/wizard-state.svelte.js';

  const roots = $derived(wizardState.roots.roots);

  function addRoot() {
    const trimmed = wizardState.roots.newPath.trim();
    if (!trimmed || roots.some(r => r.path === trimmed)) return;
    wizardState.roots.roots = [...roots, {
      id: `root-${Date.now()}`, path: trimmed, name: trimmed.split('/').pop() ?? trimmed,
      status: 'scanning' as const, excluded: [], repos_found: 0, scanned: false,
      modified_at: new Date().toISOString(),
    }];
    wizardState.roots.newPath = '';
  }

  function removeRoot(id: string) {
    wizardState.roots.roots = roots.filter(r => r.id !== id);
  }
</script>

<div class="step">
  <p class="step-desc">Where your work lives. Sensei recurses and finds repositories.</p>

  <div class="input-row">
    <input
      type="text"
      class="folder-input"
      bind:value={wizardState.roots.newPath}
      onkeydown={(e) => { if (e.key === 'Enter') addRoot(); }}
      placeholder="~/Developer"
    />
    <button class="btn-solid" onclick={addRoot}>Add</button>
  </div>

  <div class="folder-list">
    {#each roots as r (r.id)}
      <div class="folder-row">
        <span class="folder-arrow">&#9656;</span>
        <div class="folder-info">
          <div class="folder-path">{r.path}</div>
          {#if r.repos_found > 0}<div class="folder-note">{r.repos_found} repositories found</div>{/if}
        </div>
        {#if r.status === 'watching'}
          <span class="chip watching">watching</span>
        {:else}
          <span class="chip">recursive</span>
        {/if}
        <button class="btn-remove" onclick={() => removeRoot(r.id)}>×</button>
      </div>
    {/each}
  </div>

  <p class="footer-note">You can manage roots and exclusions later from Settings.</p>
</div>

<style>
  .step { max-width: 780px; }
  .step-desc { font-size: 14px; color: var(--sumi-3); line-height: 1.6; margin: 0 0 24px; }

  .input-row { display: flex; gap: var(--space-2); margin-bottom: var(--space-6); }
  .folder-input {
    flex: 1; min-width: 0; padding: 8px 12px; font-size: 13px;
    font-family: var(--font-mono); color: var(--sumi);
    background: var(--paper-2); border: var(--border-card);
    border-radius: var(--radius); outline: none;
  }
  .folder-input::placeholder { color: var(--sumi-4); }
  .folder-input:focus { border-color: var(--sumi-3); }

  .folder-list { display: flex; flex-direction: column; gap: var(--space-3); margin-bottom: var(--space-8); }
  .folder-row {
    display: flex; align-items: center; gap: var(--space-3);
    padding: var(--space-4) var(--space-5);
    background: var(--paper-2); border-radius: var(--radius-lg);
  }
  .folder-arrow { font-size: 10px; color: var(--sumi-4); }
  .folder-info { flex: 1; min-width: 0; }
  .folder-path { font-size: 14px; font-family: var(--font-mono); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .folder-note { font-size: 12px; color: var(--sumi-3); margin-top: 2px; }

  .chip {
    font-size: 11px; color: var(--sumi-3); border: var(--border-card);
    border-radius: var(--radius); padding: 2px 8px; white-space: nowrap;
  }
  .chip.watching {
    color: var(--jade); border-color: var(--jade-soft); background: var(--jade-soft);
  }

  .btn-remove {
    font-size: 16px; color: var(--sumi-4); background: none; border: none;
    cursor: pointer; padding: 4px; line-height: 1;
  }
  .btn-remove:hover { color: var(--shu); }
  .footer-note { font-size: 13px; color: var(--sumi-3); }
</style>
