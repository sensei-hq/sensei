<script lang="ts">
  /**
   * AssistantCard — fully controlled card for the setup wizard's
   * Assistants step. Ports docs/mockups/lib/assistant-card.jsx to
   * Svelte 5. The card holds no state of its own: a parent (the
   * wizard's AssistantState slice) hands the status of every part
   * down, so one common reducer drives any number of cards.
   *
   * Part status vocabulary (mirrors AssistantPartStatus from the
   * daemon SSE contract):
   *   idle        — empty ring (switch off / nothing yet)
   *   configuring — spinner (in progress)
   *   done        — check (registered)
   *   error       — cross (failed; see consolidated `error`)
   */
  import AssistantBrandMark from './AssistantBrandMark.svelte';
  import type { AssistantPartStatus } from '$lib/types.js';

  interface PartView {
    id: string;
    label: string;
    status: AssistantPartStatus;
  }

  let {
    id, name, found = true, enabled = false,
    parts = [], error = null,
    onToggle, onRetry,
  }: {
    id: string;
    name: string;
    found?: boolean;
    enabled?: boolean;
    parts?: PartView[];
    /** Consolidated family-level error — the parent derives this by
     *  joining `partErrors` for parts whose status is `error`. */
    error?: string | null;
    onToggle?: () => void;
    onRetry?: () => void;
  } = $props();

  const busy = $derived(enabled && parts.some(p => p.status === 'configuring'));
  const showError = $derived(enabled && !!error && !busy);

  /** Direct click on the switch button. Disabled (no-op) while the
   *  family is busy or has no installed variants — the daemon would
   *  reject either path anyway. */
  function handleSwitchClick() {
    if (!found || busy) return;
    onToggle?.();
  }

  /** Header status label — mirrors mockup's headerStatus() derivation. */
  function headerLabel(): { text: string; tone: string; icon: 'spinner' | 'check' | 'cross' | null } {
    if (!found)   return { text: 'not found',     tone: 'muted-italic',    icon: null };
    if (!enabled) return { text: 'off',           tone: 'muted',           icon: null };
    if (parts.some(p => p.status === 'configuring'))
                  return { text: 'configuring…',  tone: 'accent',          icon: 'spinner' };
    if (parts.some(p => p.status === 'error'))
                  return { text: 'failed',        tone: 'danger',          icon: 'cross' };
    if (parts.length && parts.every(p => p.status === 'done'))
                  return { text: 'configured',    tone: 'success',         icon: 'check' };
    return { text: '', tone: 'muted', icon: null };
  }
  const status = $derived(headerLabel());
</script>

<div
  class="card"
  class:not-found={!found}
  data-testid={`assistant-card-${id}`}
  data-found={found}
  data-enabled={enabled}
>
  <div class="header">
    <div class="logo-tile" class:dim={!enabled && found}>
      <AssistantBrandMark {id} {name} size={21} />
    </div>

    <span class="title">{name}</span>

    <div class="meta">
      {#if status.text}
        <span class="status status-{status.tone}">
          {#if status.icon === 'spinner'}
            <svg class="spin" width="11" height="11" viewBox="0 0 16 16" aria-hidden="true">
              <circle cx="8" cy="8" r="6" fill="none" stroke="currentColor" stroke-width="2" stroke-opacity="0.25"/>
              <path d="M8 2 a6 6 0 0 1 6 6" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
            </svg>
          {:else if status.icon === 'check'}
            <svg width="11" height="11" viewBox="0 0 16 16" aria-hidden="true">
              <path d="M3.5 8.5 L6.5 11.5 L12.5 4.5" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          {:else if status.icon === 'cross'}
            <svg width="11" height="11" viewBox="0 0 16 16" aria-hidden="true">
              <path d="M4.5 4.5 L11.5 11.5 M11.5 4.5 L4.5 11.5" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round"/>
            </svg>
          {/if}
          {status.text}
        </span>
      {/if}

      <button
        type="button"
        class="switch"
        class:switch-on={enabled}
        disabled={!found || busy}
        role="switch"
        aria-checked={enabled}
        aria-label={`Enable ${name}`}
        onclick={handleSwitchClick}
      >
        <span class="switch-knob" class:switch-knob-on={enabled}></span>
      </button>
    </div>
  </div>

  <div class="chips">
    {#each parts as p (p.id)}
      <span
        class="chip chip-{enabled ? p.status : 'idle'}"
        data-part={p.id}
        data-status={enabled ? p.status : 'idle'}
      >
        {#if !enabled || p.status === 'idle'}
          <span class="chip-ring" aria-hidden="true"></span>
        {:else if p.status === 'configuring'}
          <svg class="spin" width="11" height="11" viewBox="0 0 16 16" aria-hidden="true">
            <circle cx="8" cy="8" r="6" fill="none" stroke="currentColor" stroke-width="2" stroke-opacity="0.25"/>
            <path d="M8 2 a6 6 0 0 1 6 6" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
          </svg>
        {:else if p.status === 'done'}
          <svg width="11" height="11" viewBox="0 0 16 16" aria-hidden="true">
            <path d="M3.5 8.5 L6.5 11.5 L12.5 4.5" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        {:else if p.status === 'error'}
          <svg width="11" height="11" viewBox="0 0 16 16" aria-hidden="true">
            <path d="M4.5 4.5 L11.5 11.5 M11.5 4.5 L4.5 11.5" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round"/>
          </svg>
        {/if}
        {p.label}
      </span>
    {/each}
  </div>

  {#if showError}
    <div class="error-block" role="alert">
      <svg width="11" height="11" viewBox="0 0 16 16" aria-hidden="true" class="error-icon">
        <path d="M4.5 4.5 L11.5 11.5 M11.5 4.5 L4.5 11.5" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round"/>
      </svg>
      <div class="error-text">
        Couldn’t configure {name} — <span class="mono error-message">{error}</span>
      </div>
      {#if onRetry}
        <button type="button" class="retry" onclick={() => onRetry?.()}>Retry →</button>
      {/if}
    </div>
  {/if}
</div>

<style>
  .card {
    display: flex;
    flex-direction: column;
    gap: 11px;
    padding: 15px 18px;
    border: 1px solid oklch(var(--color-surface-z3) / 1);
    border-radius: 10px;
    background: oklch(var(--color-surface-z1) / 1);
    transition: opacity 180ms ease, background 180ms ease, border-color 180ms ease;
  }
  .card.not-found {
    border-style: dashed;
    background: transparent;
    opacity: 0.55;
  }

  .header {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .logo-tile {
    width: 34px;
    height: 34px;
    flex-shrink: 0;
    border-radius: 8px;
    border: 1px solid oklch(var(--color-surface-z3) / 1);
    background: oklch(var(--color-surface-z0) / 1);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: oklch(var(--color-surface-z9) / 1);
  }
  .logo-tile.dim { color: oklch(var(--color-surface-z6) / 1); }

  .title {
    font-size: 15px;
    font-weight: 600;
  }

  .meta {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .status {
    font-size: 11px;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    white-space: nowrap;
    flex-shrink: 0;
    font-family: var(--font-mono);
  }
  .status-muted        { color: oklch(var(--color-surface-z6) / 1); }
  .status-muted-italic { color: oklch(var(--color-surface-z6) / 1); font-style: italic; font-family: var(--font-ui); }
  .status-accent       { color: oklch(var(--color-primary-z6) / 1); }
  .status-success      { color: oklch(var(--color-success-z6) / 1); }
  .status-danger       { color: oklch(var(--color-danger-z5) / 1); }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-family: var(--font-mono);
    font-size: 11px;
    padding: 3px 10px 3px 8px;
    border-radius: 999px;
    border: 1px solid oklch(var(--color-surface-z3) / 1);
    background: transparent;
    color: oklch(var(--color-surface-z6) / 1);
    white-space: nowrap;
    transition: color 180ms ease, background 180ms ease, border-color 180ms ease;
  }
  .chip-idle {
    border-style: dashed;
  }
  .chip-configuring {
    color: oklch(var(--color-primary-z6) / 1);
    background: oklch(var(--color-surface-z2) / 1);
    border-color: oklch(var(--color-primary-z4) / 0.5);
  }
  .chip-done {
    color: oklch(var(--color-success-z6) / 1);
    background: oklch(var(--color-success-z6) / 0.10);
    border-color: oklch(var(--color-success-z6) / 0.30);
  }
  .chip-error {
    color: oklch(var(--color-danger-z5) / 1);
    background: oklch(var(--color-danger-z5) / 0.10);
    border-color: oklch(var(--color-danger-z5) / 0.32);
  }
  .chip-ring {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    border: 1.5px solid currentColor;
    display: inline-block;
  }

  .error-block {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 9px 11px;
    border-radius: 7px;
    background: oklch(var(--color-danger-z5) / 0.10);
    border: 1px solid oklch(var(--color-danger-z5) / 0.32);
  }
  .error-icon {
    color: oklch(var(--color-danger-z5) / 1);
    margin-top: 1px;
    flex-shrink: 0;
  }
  .error-text {
    min-width: 0;
    flex: 1;
    font-size: 12.5px;
    color: oklch(var(--color-surface-z7) / 1);
    line-height: 1.45;
  }
  .error-message {
    color: oklch(var(--color-danger-z5) / 1);
  }
  .retry {
    font-family: var(--font-mono);
    font-size: 11px;
    color: oklch(var(--color-primary-z6) / 1);
    background: transparent;
    border: 0;
    cursor: pointer;
    flex-shrink: 0;
    margin-top: 1px;
    padding: 0;
  }

  .spin {
    animation: zs-spin 0.9s linear infinite;
    transform-origin: 50% 50%;
  }
  @keyframes zs-spin {
    from { transform: rotate(0deg); }
    to   { transform: rotate(360deg); }
  }

  .mono { font-family: var(--font-mono); }

  .switch {
    width: 38px;
    height: 22px;
    border-radius: 999px;
    flex-shrink: 0;
    padding: 0;
    position: relative;
    cursor: pointer;
    border: 1px solid oklch(var(--color-surface-z3) / 1);
    background: oklch(var(--color-surface-z3) / 1);
    transition: background 180ms ease, opacity 180ms ease;
  }
  .switch:disabled {
    cursor: default;
    opacity: 0.6;
  }
  .switch.switch-on {
    background: oklch(var(--color-ink-z9) / 1);
    border-color: transparent;
  }
  .switch-knob {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: oklch(var(--color-surface-z0) / 1);
    box-shadow: 0 1px 2px oklch(0 0 0 / 0.15);
    transition: left 180ms ease;
  }
  .switch-knob.switch-knob-on {
    left: 18px;
  }
</style>
