<script lang="ts">
  import { onMount } from 'svelte';
  import type { WizardState, WizUpdate } from '../types.js';
  import { getPort } from '$lib/appstate.svelte.js';
  import {
    checkComponents, installViaBrew,
    getInitialComponents,
    type ComponentInfo, type ComponentState,
  } from '../daemon.js';

  let { wizState, update }: {
    wizState: WizardState;
    update: WizUpdate;
  } = $props();

  const port = $derived(getPort());

  let components = $state<ComponentInfo[]>(getInitialComponents());
  let allReady = $derived(components.every(c => c.state === 'ready'));
  let anyMissing = $derived(components.some(c => c.state === 'missing'));
  let installStatus = $state<string | null>(null);

  onMount(() => {
    checkComponents((updated) => {
      components = updated;
      // Sync to wizard state
      update({
        components: updated.map(c => ({
          id: c.id, name: c.name, icon: c.icon,
          version: c.version, status: c.state,
        })),
      });
    });
  });

  async function handleInstall() {
    installStatus = 'Installing...';
    const ok = await installViaBrew((status) => { installStatus = status; });
    if (ok) {
      // Re-check after install
      checkComponents((updated) => { components = updated; });
    }
  }

  function stateLabel(state: ComponentState): string {
    switch (state) {
      case 'checking': return 'checking...';
      case 'missing': return 'not found';
      case 'installing': return 'installing...';
      case 'stopped': return 'stopped';
      case 'starting': return 'starting...';
      case 'ready': return 'ready';
      case 'error': return 'error';
    }
  }

  function stateColor(state: ComponentState): string {
    switch (state) {
      case 'ready': return 'var(--jade)';
      case 'checking': case 'starting': case 'installing': return 'var(--amber)';
      case 'missing': case 'error': return 'var(--shu)';
      case 'stopped': return 'var(--sumi-3)';
    }
  }
</script>

<section class="step">
  <div class="step-label"><span class="kanji">二</span> STEP</div>
  <h1 class="display headline">Components</h1>
  <p class="subtitle">
    {#if allReady}
      Everything is in place.
    {:else if anyMissing}
      Some components need to be installed.
    {:else}
      Checking components...
    {/if}
  </p>

  <div class="cards">
    {#each components as comp}
      <div class="card" data-state={comp.state}>
        <div class="card-icon kanji">{comp.icon}</div>
        <div class="card-info">
          <div class="card-name">{comp.name}</div>
          <div class="card-meta">
            {#if comp.version}
              <span>{comp.version}</span>
              <span class="dot">&middot;</span>
            {/if}
            <span>{stateLabel(comp.state)}</span>
            {#if comp.error}
              <span class="dot">&middot;</span>
              <span class="error-hint">{comp.error}</span>
            {/if}
          </div>
        </div>
        <div class="badge" style="color: {stateColor(comp.state)}">
          {#if comp.state === 'ready'}
            <span class="badge-dot" style="background: {stateColor(comp.state)}"></span>
          {:else if comp.state === 'checking' || comp.state === 'starting' || comp.state === 'installing'}
            <span class="spinner"></span>
          {/if}
          {stateLabel(comp.state).toUpperCase()}
        </div>
      </div>
    {/each}
  </div>

  {#if anyMissing}
    <div class="install-section">
      {#if installStatus}
        <p class="install-status">{installStatus}</p>
      {:else}
        <p class="install-hint">Install all components:</p>
        <code class="install-cmd">brew install mizukisu/tap/sensei</code>
        <button class="btn-solid" onclick={handleInstall}>Install via Homebrew</button>
      {/if}
    </div>
  {/if}

  <p class="footer-note">Nothing leaves <code>localhost:{port}</code>.</p>
</section>

<style>
  .step { max-width: 780px; }
  .step-label { font-size: 12px; letter-spacing: 0.12em; color: var(--sumi-3); margin-bottom: var(--space-2); }
  .step-label .kanji { color: var(--shu); margin-right: 4px; }
  .headline { font-size: 40px; color: var(--sumi); margin: 0 0 var(--space-2) 0; line-height: 1.15; }
  .subtitle { font-size: 15px; color: var(--sumi-3); margin: 0 0 var(--space-8) 0; }
  .cards { display: flex; flex-direction: column; gap: var(--space-3); }

  .card {
    display: flex; align-items: center; gap: var(--space-4);
    padding: var(--space-5) var(--space-6);
    background: var(--paper-2); border-radius: var(--radius-lg);
    transition: opacity 0.2s;
  }
  .card[data-state="checking"], .card[data-state="starting"] { opacity: 0.7; }
  .card[data-state="missing"] { opacity: 0.5; }

  .card-icon {
    width: 40px; height: 40px;
    display: flex; align-items: center; justify-content: center;
    font-size: 18px; color: var(--sumi-3);
    border: var(--border-card); border-radius: var(--radius);
    background: var(--paper); flex-shrink: 0;
  }
  .card-info { flex: 1; min-width: 0; }
  .card-name { font-size: 15px; font-weight: 500; color: var(--sumi); }
  .card-meta { font-size: 13px; color: var(--sumi-3); margin-top: 2px; }
  .dot { margin: 0 4px; }
  .error-hint { color: var(--shu); font-size: 11px; }

  .badge {
    display: flex; align-items: center; gap: 6px;
    font-size: 11px; font-weight: 600; letter-spacing: 0.06em;
    flex-shrink: 0;
  }
  .badge-dot { width: 8px; height: 8px; border-radius: 50%; }

  .spinner {
    width: 12px; height: 12px;
    border: 2px solid var(--paper-edge);
    border-top-color: currentColor;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin { to { transform: rotate(360deg); } }

  .install-section {
    margin-top: var(--space-8);
    padding: var(--space-6);
    background: var(--paper-2);
    border-radius: var(--radius-lg);
    border: var(--border-card);
  }
  .install-hint { font-size: 13px; color: var(--sumi-2); margin-bottom: var(--space-3); }
  .install-cmd {
    display: block;
    font-family: var(--font-mono); font-size: 13px;
    color: var(--sumi); background: var(--paper-3);
    padding: var(--space-3) var(--space-4);
    border-radius: var(--radius);
    margin-bottom: var(--space-4);
    user-select: all;
  }
  .install-status { font-size: 13px; color: var(--sumi-3); }

  .footer-note { margin-top: var(--space-8); font-size: 13px; color: var(--sumi-3); }
  .footer-note code { font-family: var(--font-mono); font-size: 12px; }
</style>
