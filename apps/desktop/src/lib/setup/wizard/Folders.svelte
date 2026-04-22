<script lang="ts">
  import type { WizardState, WizUpdate, ScanFolder } from '../types.js';

  let { wizState, update }: {
    wizState: WizardState;
    update: WizUpdate;
  } = $props();

  let inputValue = $state('');

  function addFolder() {
    const trimmed = inputValue.trim();
    if (!trimmed) return;
    // Avoid duplicates
    if (wizState.folders.some(f => f.path === trimmed)) return;
    const newFolder: ScanFolder = {
      id: `f${Date.now()}`,
      path: trimmed,
      note: '',
    };
    update({ folders: [...wizState.folders, newFolder] });
    inputValue = '';
  }

  function removeFolder(id: string) {
    update({ folders: wizState.folders.filter(f => f.id !== id) });
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') addFolder();
  }
</script>

<section class="step">
  <div class="step-label"><span class="kanji">四</span> STEP</div>
  <h1 class="display headline">Folders</h1>
  <p class="subtitle">Where your work lives. Sensei recurses and finds repos.</p>

  <div class="input-row">
    <input
      type="text"
      class="folder-input"
      bind:value={inputValue}
      onkeydown={onKeydown}
      placeholder="~/code/my-project"
    />
    <button class="btn-add" onclick={addFolder}>Add</button>
    <button class="btn-browse" onclick={addFolder}>Browse...</button>
  </div>

  <div class="folder-list">
    {#each wizState.folders as folder}
      <div class="folder-row">
        <span class="folder-arrow">&#9656;</span>
        <div class="folder-info">
          <div class="folder-path">{folder.path}</div>
          {#if folder.note}
            <div class="folder-note">{folder.note}</div>
          {/if}
        </div>
        <span class="chip-recursive">recursive</span>
        <button class="link-remove" onclick={() => removeFolder(folder.id)}>remove</button>
      </div>
    {/each}
  </div>

  <p class="footer-note">You can manage folders and exclusions later from Settings.</p>
</section>

<style>
  .step {
    padding: var(--space-10) var(--space-12);
    max-width: 780px;
  }

  .step-label {
    font-size: 12px;
    letter-spacing: 0.12em;
    color: var(--sumi-3);
    margin-bottom: var(--space-2);
  }

  .step-label .kanji {
    color: var(--shu);
    margin-right: 4px;
  }

  .headline {
    font-size: 40px;
    color: var(--sumi);
    margin: 0 0 var(--space-2) 0;
    line-height: 1.15;
  }

  .subtitle {
    font-size: 15px;
    color: var(--sumi-3);
    margin: 0 0 var(--space-8) 0;
  }

  .input-row {
    display: flex;
    gap: var(--space-2);
    margin-bottom: var(--space-6);
  }

  .folder-input {
    flex: 1;
    min-width: 0;
    padding: var(--space-3) var(--space-4);
    font-size: 14px;
    font-family: var(--font-mono);
    color: var(--sumi);
    background: var(--paper);
    border: var(--hairline);
    border-radius: var(--radius-lg);
    outline: none;
    transition: border-color 0.14s;
  }

  .folder-input::placeholder {
    color: var(--sumi-4);
  }

  .folder-input:focus {
    border-color: var(--sumi-3);
  }

  .btn-add {
    padding: var(--space-3) var(--space-5);
    font-size: 14px;
    font-weight: 500;
    color: var(--paper);
    background: var(--sumi);
    border: none;
    border-radius: var(--radius-lg);
    cursor: pointer;
    font-family: var(--font-ui);
    transition: opacity 0.14s;
    white-space: nowrap;
  }

  .btn-add:hover {
    opacity: 0.85;
  }

  .btn-browse {
    padding: var(--space-3) var(--space-5);
    font-size: 14px;
    font-weight: 500;
    color: var(--sumi);
    background: var(--paper);
    border: var(--hairline);
    border-radius: var(--radius-lg);
    cursor: pointer;
    font-family: var(--font-ui);
    transition: background 0.14s;
    white-space: nowrap;
  }

  .btn-browse:hover {
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
    border: var(--hairline);
    border-radius: var(--radius);
    padding: 2px 8px;
    white-space: nowrap;
    flex-shrink: 0;
  }

  .link-remove {
    font-size: 13px;
    color: var(--sumi-3);
    background: none;
    border: none;
    cursor: pointer;
    padding: 0;
    font-family: var(--font-ui);
    transition: color 0.14s;
    flex-shrink: 0;
  }

  .link-remove:hover {
    color: var(--shu);
  }

  .footer-note {
    font-size: 13px;
    color: var(--sumi-3);
  }
</style>
