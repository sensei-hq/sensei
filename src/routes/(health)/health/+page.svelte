<script lang="ts">
  import { goto } from '$app/navigation';
  import { appState } from '$lib/appstate.svelte.js';
  import { runBootstrap, installComponent, startComponent, createDatabase } from '$lib/bootstrap.js';
  import { stateLabel, isReady as stateIsReady, isFailed, errorMessage, type ComponentStatus, type ComponentState } from '$lib/bootstrap.js';
  import * as state from '$lib/bootstrap-state.svelte.js';

  // ── Lifecycle ──────────────────────────────────────────────

  async function check() {
    state.setLoading(true);
    const result = await runBootstrap();
    state.applyResult(result);

    if (state.isReady()) advance();
  }

  function advance() {
    if (appState.setupComplete) {
      goto('/observatory', { replaceState: true });
    } else {
      goto('/config', { replaceState: true });
    }
  }

  $effect(() => { check(); });

  // ── Actions (call API, feed results to state) ──────────────

  async function handleInstall(name: string) {
    state.setActionInProgress(name);
    try {
      const result = await installComponent(name);
      state.updateComponent(name, result);
    } catch { /* re-check will show status */ }
    state.setActionInProgress(null);
    await check();
  }

  async function handleStart(name: string) {
    state.setActionInProgress(name);
    try {
      const result = await startComponent(name);
      state.updateComponent(name, result);
    } catch { /* re-check will show status */ }
    state.setActionInProgress(null);
    await check();
  }

  async function handleCreateDb() {
    state.setActionInProgress('database');
    try {
      const result = await createDatabase();
      state.updateComponent('database', result);
    } catch { /* re-check will show status */ }
    state.setActionInProgress(null);
    await check();
  }

  async function handleSkip(name: string) {
    state.skipComponent(name);
    if (state.isReady()) advance();
  }

  // ── Visual helpers ─────────────────────────────────────────

  function stateColor(s: ComponentState): string {
    switch (s.state) {
      case 'ready': return 'var(--jade)';
      case 'failed': return 'var(--amber)';
      case 'skipped': return 'var(--sumi-4)';
      default: return 'var(--shu)';
    }
  }

  function stateIcon(s: ComponentState): string {
    switch (s.state) {
      case 'ready': return '\u25CF';
      case 'failed': return '\u25C6';
      case 'skipped': return '\u25CB';
      default: return '\u25CE';
    }
  }

  function isPulsing(s: ComponentState): boolean {
    return s.state === 'detecting' || s.state === 'installing' || s.state === 'starting';
  }

  function isSkippable(name: string): boolean {
    return name === 'ollama' || name.startsWith('gemma') || name.startsWith('qwen');
  }

  function actionLabel(comp: ComponentStatus): string | null {
    if (!isFailed(comp.state)) return null;
    const err = errorMessage(comp.state) ?? '';
    if (err.includes('not installed')) return 'Install';
    if (err.includes('not reachable') || err.includes('not running')) return 'Start';
    if (err.includes('does not exist')) return 'Create';
    return 'Fix';
  }

  function handleAction(comp: ComponentStatus) {
    const label = actionLabel(comp);
    if (label === 'Install') handleInstall(comp.name);
    else if (label === 'Start') handleStart(comp.name);
    else if (label === 'Create') handleCreateDb();
    else handleInstall(comp.name);
  }

  // ── Read from state ────────────────────────────────────────

  const components = $derived(state.getComponents());
  const hardware = $derived(state.getHardware());
  const loading = $derived(state.isLoading());
  const ready = $derived(state.isReady());
  const actionInProgress = $derived(state.getActionInProgress());
</script>

<div class="bootstrap">
  <span class="kanji hero-kanji">整</span>

  <h1 class="display hero-title">
    {#if loading}
      Checking prerequisites…
    {:else if ready}
      All set.
    {:else}
      Getting ready.
    {/if}
  </h1>

  <p class="hero-sub">
    {#if loading}
      Verifying components.
    {:else if ready}
      Everything looks healthy.
    {:else}
      Some components need attention.
    {/if}
  </p>

  {#if components.length > 0}
    <div class="components">
      {#each components as comp (comp.name)}
        <div class="comp-row">
          <span class="comp-dot" style="color: {stateColor(comp.state)}">
            {#if isPulsing(comp.state)}
              <span class="pulse">{stateIcon(comp.state)}</span>
            {:else}
              {stateIcon(comp.state)}
            {/if}
          </span>

          <div class="comp-info">
            <span class="comp-name">{comp.name}</span>
            {#if comp.version}
              <span class="comp-version">{comp.version}</span>
            {/if}
            <span class="comp-state" style="color: {stateColor(comp.state)}">
              {stateLabel(comp.state)}
            </span>
            {#if isFailed(comp.state)}
              <span class="comp-error">{errorMessage(comp.state)}</span>
            {/if}
          </div>

          <div class="comp-actions">
            {#if isFailed(comp.state) && actionLabel(comp)}
              <button
                class="btn-outline btn-sm"
                onclick={() => handleAction(comp)}
                disabled={actionInProgress === comp.name}
              >
                {actionInProgress === comp.name ? 'Working…' : actionLabel(comp)}
              </button>
            {/if}
            {#if isFailed(comp.state) && isSkippable(comp.name)}
              <button class="btn-text btn-sm" onclick={() => handleSkip(comp.name)}>
                Skip
              </button>
            {/if}
          </div>
        </div>
      {/each}
    </div>

    {#if hardware.ram_gb > 0}
      <div class="hw-info">
        {hardware.ram_gb}GB RAM · {hardware.cpu_cores} cores
        {#if hardware.gpu} · {hardware.gpu}{/if}
        · {hardware.recommended_tier} tier
      </div>
    {/if}
  {/if}

  {#if !loading && !ready}
    <button class="btn-solid" onclick={check} style="margin-top: 24px;">
      Recheck
    </button>
  {/if}

  <p class="hint">Nothing leaves localhost.</p>
</div>

<style>
  .bootstrap { text-align: center; max-width: 520px; width: 100%; }
  .hero-kanji { font-size: 56px; color: var(--shu); opacity: 0.3; display: block; margin-bottom: 16px; }
  .hero-title { font-size: 26px; font-weight: 300; margin: 0 0 8px; letter-spacing: -0.01em; }
  .hero-sub { font-size: 13px; color: var(--sumi-3); margin: 0 0 28px; }

  .components { text-align: left; border: var(--hairline); border-radius: var(--radius-lg); overflow: hidden; background: var(--paper); }
  .comp-row { display: flex; align-items: center; gap: 12px; padding: 12px 16px; border-bottom: var(--ink-line); }
  .comp-row:last-child { border-bottom: none; }
  .comp-dot { font-size: 12px; flex-shrink: 0; width: 16px; text-align: center; }
  .comp-info { flex: 1; display: flex; align-items: baseline; gap: 8px; flex-wrap: wrap; min-width: 0; }
  .comp-name { font-family: var(--font-mono); font-size: 12px; font-weight: 500; color: var(--sumi); }
  .comp-version { font-family: var(--font-mono); font-size: 11px; color: var(--sumi-3); }
  .comp-state { font-size: 11px; letter-spacing: 0.03em; }
  .comp-error { font-size: 11px; color: var(--amber); width: 100%; }
  .comp-actions { display: flex; gap: 6px; flex-shrink: 0; }

  .hw-info { margin-top: 16px; font-size: 11px; color: var(--sumi-4); letter-spacing: 0.02em; }
  .btn-sm { padding: 4px 10px; font-size: 11px; }
  .btn-text { background: none; border: none; color: var(--sumi-3); font-size: 11px; cursor: pointer; padding: 4px 8px; font-family: var(--font-ui); }
  .btn-text:hover { color: var(--sumi-2); }
  .hint { margin-top: 20px; font-size: 10px; color: var(--sumi-4); letter-spacing: 0.08em; text-transform: uppercase; }
  .pulse { animation: pulse 1.2s ease-in-out infinite; }
  @keyframes pulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.3; } }
</style>
