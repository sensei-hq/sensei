<script lang="ts">
  import { onMount } from 'svelte';
  import type { WizardState, WizUpdate, ScanFolder, WizStage } from '../types.js';
  import { senseiApi } from '$lib/api.js';
  import { getPort } from '$lib/appstate.svelte.js';
  import StepHeader from './StepHeader.svelte';

  let { wizState, update, stage }: {
    wizState: WizardState;
    update: WizUpdate;
    stage: WizStage;
  } = $props();

  let inputValue = $state('');

  // Local reactive state — loaded from daemon, not from parent props
  let folders = $state<ScanFolder[]>([]);
  let scannedPaths = $state<Set<string>>(new Set());

  // Load existing scan roots from daemon
  onMount(async () => {
    try {
      const api = senseiApi(getPort());
      const roots = await api.getScanRoots();
      if (roots.length > 0) {
        folders = roots.map((r, i) => ({
          id: `existing-${i}`,
          path: r.path,
          note: r.scanned ? `${r.repos_found} repos found` : '',
        }));
        scannedPaths = new Set(roots.filter(r => r.scanned).map(r => r.path));
        update({ folders });
      }
    } catch { /* daemon not available */ }
  });

  function addFolder() {
    const trimmed = inputValue.trim();
    if (!trimmed) return;
    if (folders.some(f => f.path === trimmed)) return;
    folders = [...folders, { id: `f${Date.now()}`, path: trimmed, note: '' }];
    update({ folders });
    inputValue = '';
  }

  function removeFolder(id: string) {
    folders = folders.filter(f => f.id !== id);
    update({ folders });
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') addFolder();
  }
</script>

<section class="step">
  <StepHeader {stage} subtitle="Where your work lives. Sensei recurses and finds repos." />

  <div class="input-row">
    <input
      type="text"
      class="folder-input"
      bind:value={inputValue}
      onkeydown={onKeydown}
      placeholder="~/code/my-project"
    />
    <button class="btn-solid" onclick={addFolder}>Add</button>
    <button class="btn-outline" onclick={addFolder}>Browse...</button>
  </div>

  <div class="folder-list">
    {#each folders as folder}
      <div class="folder-row">
        <span class="folder-arrow">&#9656;</span>
        <div class="folder-info">
          <div class="folder-path">{folder.path}</div>
          {#if folder.note}
            <div class="folder-note">{folder.note}</div>
          {/if}
        </div>
        {#if scannedPaths.has(folder.path)}
          <span class="chip-watching">watching</span>
        {:else}
          <span class="chip-recursive">recursive</span>
        {/if}
        <button class="btn-remove" onclick={() => removeFolder(folder.id)} title="Remove folder">×</button>
      </div>
    {/each}
  </div>

  <p class="footer-note">You can manage folders and exclusions later from Settings.</p>
</section>

<style>
  .step {
    max-width: 780px;
  }

  .input-row {
    display: flex;
    gap: var(--space-2);
    margin-bottom: var(--space-6);
  }

  .folder-input {
    flex: 1;
    min-width: 0;
    padding: 8px 12px;
    font-size: 13px;
    font-family: var(--font-mono);
    color: var(--sumi);
    background: var(--paper-2);
    border: var(--border-card);
    border-radius: var(--radius);
    outline: none;
    transition: border-color 0.14s;
  }

  .folder-input::placeholder {
    color: var(--sumi-4);
  }

  .folder-input:focus {
    border-color: var(--sumi-3);
  }

  .btn-solid {
    padding: 8px 16px;
    font-size: 13px;
    font-weight: 500;
    color: var(--paper);
    background: var(--sumi);
    border: none;
    border-radius: var(--radius);
    cursor: pointer;
    font-family: var(--font-ui);
    transition: opacity 0.14s;
    white-space: nowrap;
  }

  .btn-solid:hover {
    opacity: 0.85;
  }

  .btn-outline {
    padding: 8px 16px;
    font-size: 13px;
    font-weight: 500;
    color: var(--sumi);
    background: transparent;
    border: var(--border-card);
    border-radius: var(--radius);
    cursor: pointer;
    font-family: var(--font-ui);
    transition: background 0.14s;
    white-space: nowrap;
  }

  .btn-outline:hover {
    background: var(--paper-2);
  }

  .folder-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    margin-bottom: var(--space-8);
  }

  .folder-row {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-4) var(--space-5);
    background: var(--paper-2);
    border-radius: var(--radius-lg);
  }

  .folder-arrow {
    font-size: 10px;
    color: var(--sumi-4);
    flex-shrink: 0;
  }

  .folder-info {
    flex: 1;
    min-width: 0;
  }

  .folder-path {
    font-size: 14px;
    font-family: var(--font-mono);
    color: var(--sumi);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .folder-note {
    font-size: 12px;
    color: var(--sumi-3);
    margin-top: 2px;
  }

  .chip-recursive {
    font-size: 11px;
    color: var(--sumi-3);
    border: var(--border-card);
    border-radius: var(--radius);
    padding: 2px 8px;
    white-space: nowrap;
    flex-shrink: 0;
  }

  .chip-watching {
    font-size: 11px;
    color: var(--jade);
    border: 1px solid var(--jade-soft);
    background: var(--jade-soft);
    border-radius: var(--radius);
    padding: 2px 8px;
    white-space: nowrap;
    flex-shrink: 0;
  }

  .btn-remove {
    font-size: 16px;
    color: var(--sumi-4);
    background: none;
    border: none;
    cursor: pointer;
    padding: 4px;
    line-height: 1;
    transition: color 0.14s;
    flex-shrink: 0;
  }

  .btn-remove:hover {
    color: var(--shu);
  }

  .footer-note {
    font-size: 13px;
    color: var(--sumi-3);
  }
</style>
