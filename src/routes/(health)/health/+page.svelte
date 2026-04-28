<script lang="ts">
  import { goto } from '$app/navigation';
  import { isSetupComplete } from '$lib/appstate.svelte.js';
  import {
    runBootstrap, installComponent, startComponent, createDatabase,
    type BootstrapResult, type ComponentStatus, type ComponentState,
    stateLabel, isReady, isFailed, errorMessage,
  } from '$lib/bootstrap.js';

  let result = $state<BootstrapResult | null>(null);
  let loading = $state(true);
  let actionInProgress = $state<string | null>(null);

  // ── Lifecycle ──────────────────────────────────────────────

  async function check() {
    loading = true;
    result = await runBootstrap();
    loading = false;

    if (result.ready) {
      advance();
    }
  }

  function advance() {
    if (isSetupComplete()) {
      goto('/observatory', { replaceState: true });
    } else {
      goto('/config', { replaceState: true });
    }
  }

  // Auto-check on mount
  $effect(() => { check(); });

  // ── Actions ────────────────────────────────────────────────

  async function handleInstall(name: string) {
    actionInProgress = name;
    try {
      await installComponent(name);
    } catch { /* re-check will show status */ }
    actionInProgress = null;
    await check();
  }

  async function handleStart(name: string) {
    actionInProgress = name;
    try {
      await startComponent(name);
    } catch { /* re-check will show status */ }
    actionInProgress = null;
    await check();
  }

  async function handleCreateDb() {
    actionInProgress = 'database';
    try {
      await createDatabase();
    } catch { /* re-check will show status */ }
    actionInProgress = null;
    await check();
  }

  async function handleSkip(name: string) {
    if (!result) return;
    const idx = result.components.findIndex(c => c.name === name);
    if (idx >= 0) {
      result.components[idx].state = { state: 'skipped' };
      result = { ...result };
      // Check if all ready/skipped now
      const allDone = result.components.every(c =>
        isReady(c.state) || c.state.state === 'skipped'
      );
      if (allDone) {
        result.ready = true;
        advance();
      }
    }
  }

  // ── Visual helpers ─────────────────────────────────────────

  function stateColor(s: ComponentState): string {
    switch (s.state) {
      case 'ready': return 'var(--jade)';
      case 'failed': return 'var(--amber)';
      case 'skipped': return 'var(--sumi-4)';
      case 'detecting':
      case 'installing':
      case 'starting':
      case 'upgrading':
      case 'pulling': return 'var(--shu)';
      default: return 'var(--sumi-3)';
    }
  }

  function stateIcon(s: ComponentState): string {
    switch (s.state) {
      case 'ready': return '\u25CF';    // ●
      case 'failed': return '\u25C6';   // ◆
      case 'skipped': return '\u25CB';  // ○
      default: return '\u25CE';         // ◎
    }
  }

  function isSkippable(name: string): boolean {
    return name === 'ollama' || name.startsWith('gemma') || name.startsWith('qwen');
  }

  function actionLabel(c: ComponentStatus): string | null {
    if (!isFailed(c.state)) return null;
    const err = errorMessage(c.state) ?? '';
    if (err.includes('not installed')) return 'Install';
    if (err.includes('not reachable') || err.includes('not running')) return 'Start';
    if (err.includes('does not exist')) return 'Create';
    return 'Fix';
  }

  function handleAction(c: ComponentStatus) {
    const label = actionLabel(c);
    if (!label) return;
    if (label === 'Install') handleInstall(c.name);
    else if (label === 'Start') handleStart(c.name);
    else if (label === 'Create') handleCreateDb();
    else handleInstall(c.name); // fallback
  }
</script>

<div class="bootstrap">
  <span class="kanji hero-kanji">整</span>

  <h1 class="display hero-title">
    {#if loading}
      Checking prerequisites…
    {:else if result?.ready}
      All set.
    {:else}
      Getting ready.
    {/if}
  </h1>

  <p class="hero-sub">
    {#if loading}
      Verifying components.
    {:else if result?.ready}
      Everything looks healthy.
    {:else}
      Some components need attention.
    {/if}
  </p>

  {#if result}
    <div class="components">
      {#each result.components as comp (comp.name)}
        <div class="comp-row">
          <span class="comp-dot" style="color: {stateColor(comp.state)}">
            {#if comp.state.state === 'installing' || comp.state.state === 'starting' || comp.state.state === 'detecting'}
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

    {#if result.hardware.ram_gb > 0}
      <div class="hw-info">
        {result.hardware.ram_gb}GB RAM · {result.hardware.cpu_cores} cores
        {#if result.hardware.gpu} · {result.hardware.gpu}{/if}
        · {result.hardware.recommended_tier} tier
      </div>
    {/if}
  {/if}

  {#if !loading && !result?.ready}
    <button class="btn-solid" onclick={check} style="margin-top: 24px;">
      Recheck
    </button>
  {/if}

  <p class="hint">Nothing leaves localhost.</p>
</div>

<style>
  .bootstrap {
    text-align: center;
    max-width: 520px;
    width: 100%;
  }

  .hero-kanji {
    font-size: 56px;
    color: var(--shu);
    opacity: 0.3;
    display: block;
    margin-bottom: 16px;
  }

  .hero-title {
    font-size: 26px;
    font-weight: 300;
    margin: 0 0 8px;
    letter-spacing: -0.01em;
  }

  .hero-sub {
    font-size: 13px;
    color: var(--sumi-3);
    margin: 0 0 28px;
  }

  /* ── Component list ──────────────────────────────── */

  .components {
    text-align: left;
    border: var(--hairline);
    border-radius: var(--radius-lg);
    overflow: hidden;
    background: var(--paper);
  }

  .comp-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 16px;
    border-bottom: var(--ink-line);
  }

  .comp-row:last-child {
    border-bottom: none;
  }

  .comp-dot {
    font-size: 12px;
    flex-shrink: 0;
    width: 16px;
    text-align: center;
  }

  .comp-info {
    flex: 1;
    display: flex;
    align-items: baseline;
    gap: 8px;
    flex-wrap: wrap;
    min-width: 0;
  }

  .comp-name {
    font-family: var(--font-mono);
    font-size: 12px;
    font-weight: 500;
    color: var(--sumi);
  }

  .comp-version {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--sumi-3);
  }

  .comp-state {
    font-size: 11px;
    letter-spacing: 0.03em;
  }

  .comp-error {
    font-size: 11px;
    color: var(--amber);
    width: 100%;
  }

  .comp-actions {
    display: flex;
    gap: 6px;
    flex-shrink: 0;
  }

  /* ── Hardware info ───────────────────────────────── */

  .hw-info {
    margin-top: 16px;
    font-size: 11px;
    color: var(--sumi-4);
    letter-spacing: 0.02em;
  }

  /* ── Button sizes ────────────────────────────────── */

  .btn-sm {
    padding: 4px 10px;
    font-size: 11px;
  }

  .btn-text {
    background: none;
    border: none;
    color: var(--sumi-3);
    font-size: 11px;
    cursor: pointer;
    padding: 4px 8px;
    font-family: var(--font-ui);
  }

  .btn-text:hover {
    color: var(--sumi-2);
  }

  .hint {
    margin-top: 20px;
    font-size: 10px;
    color: var(--sumi-4);
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  /* ── Pulse animation ─────────────────────────────── */

  .pulse {
    animation: pulse 1.2s ease-in-out infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.3; }
  }
</style>
