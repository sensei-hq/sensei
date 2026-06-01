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

  /** Coarse, e2e-friendly view of the family's overall state.
   *  Drives data-configure-state for selectors in the playwright suite —
   *  the chip strip carries the fine-grained truth, this attribute is
   *  the headline summary.
   *  In-flight `configuring` wins regardless of `enabled` so an active
   *  remove (which clears the switch first) still surfaces progress. */
  const configureState = $derived.by(() => {
    if (parts.some(p => p.status === 'configuring')) return 'configuring';
    if (!enabled) return 'idle';
    if (parts.some(p => p.status === 'error'))       return 'failed';
    if (parts.length && parts.every(p => p.status === 'done')) return 'done';
    return 'idle';
  });

  /** True iff the user-visible "configured ✓" badge would show. */
  const configured = $derived(
    enabled && parts.length > 0 && parts.every(p => p.status === 'done')
  );

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
  data-configure-state={configureState}
  data-configured={configured}
>
  <div class="header">
    <div class="logo-tile" class:dim={!enabled && found}>
      <AssistantBrandMark {id} {name} size={21} />
    </div>

    <span class="title">{name}</span>

    <div class="meta">
      {#if status.text}
        <span class="status mono status-{status.tone}">
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
        class:on={enabled}
        disabled={!found || busy}
        role="switch"
        aria-checked={enabled}
        aria-label={`Enable ${name}`}
        onclick={handleSwitchClick}
      >
        <span class="switch-knob" class:on={enabled}></span>
      </button>
    </div>
  </div>

  <div class="chips">
    {#each parts as p (p.id)}
      <!-- Status is the source of truth for the chip's visual state.
           Earlier this clamped to 'idle' when enabled=false, but that
           hid the mid-flight spinner during removal (toggle off →
           configuring → idle). The slice keeps partStatus correct
           across all flows, so trust it directly. -->
      <span
        class="chip chip-{p.status}"
        data-part={p.id}
        data-status={p.status}
      >
        {#if p.status === 'idle'}
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
  /* Local design tokens — mirror docs/mockups/lib/tokens.css exactly so
     dashes / hairlines render identical to the static mockup regardless
     of how Rokkit's `--color-surface-*` scale lands. Light defaults; the
     `body[data-mode="dark"]` override below swaps in the dark stops. */
  .card {
    --ac-paper:    oklch(0.975 0.008 85);
    --ac-paper-2:  oklch(0.940 0.010 85);
    --ac-paper-3:  oklch(0.920 0.012 85);
    --ac-edge:     oklch(0.880 0.015 85);
    --ac-ink:      oklch(0.210 0.012 50);
    --ac-ink-2:    oklch(0.320 0.012 50);
    --ac-ink-3:    oklch(0.420 0.012 50);
    --ac-ink-4:    oklch(0.600 0.012 50);
    --ac-accent:   oklch(0.625 0.180 35);
    --ac-success:  oklch(0.620 0.080 160);
    --ac-danger:   oklch(0.550 0.180 28);

    display: flex;
    flex-direction: column;
    gap: 11px;
    padding: 15px 18px;
    border: 1px solid var(--ac-edge);
    border-radius: 10px;
    background: var(--ac-paper-2);
    transition: opacity 180ms ease, background 180ms ease, border-color 180ms ease;
  }
  /* Not-found is just dimmed — same solid hairline as found cards. */
  .card.not-found {
    background: transparent;
    opacity: 0.6;
  }

  :global(body[data-mode="dark"]) .card {
    --ac-paper:    oklch(0.170 0.010 50);
    --ac-paper-2:  oklch(0.210 0.012 50);
    --ac-paper-3:  oklch(0.250 0.012 50);
    --ac-edge:     oklch(0.320 0.012 50);
    --ac-ink:      oklch(0.940 0.008 85);
    --ac-ink-2:    oklch(0.780 0.008 85);
    --ac-ink-3:    oklch(0.600 0.010 85);
    --ac-ink-4:    oklch(0.420 0.012 85);
    --ac-accent:   oklch(0.700 0.150 35);
    --ac-success:  oklch(0.720 0.090 160);
    --ac-danger:   oklch(0.620 0.180 28);
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
    border: 1px solid var(--ac-edge);
    background: var(--ac-paper);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: var(--ac-ink);
  }
  .logo-tile.dim { color: var(--ac-ink-2); }

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
  .status-muted        { color: var(--ac-ink-3); }
  .status-muted-italic { color: var(--ac-ink-3); font-style: italic; font-family: var(--font-ui); }
  .status-accent       { color: var(--ac-accent); }
  .status-success      { color: var(--ac-success); }
  .status-danger       { color: var(--ac-danger); }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  /* All chips share a solid 1px edge; only colour and bg differ between
     states (mirrors docs/mockups/lib/assistant-card.jsx CapChip skin). */
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-family: var(--font-mono);
    font-size: 11px;
    padding: 3px 10px 3px 8px;
    border-radius: 999px;
    border: 1px solid var(--ac-edge);
    background: transparent;
    color: var(--ac-ink-4);
    white-space: nowrap;
    transition: color 180ms ease, background 180ms ease, border-color 180ms ease;
  }
  .chip-configuring {
    color: var(--ac-accent);
    background: var(--ac-paper-3);
    border-color: var(--ac-edge);
  }
  .chip-done {
    color: var(--ac-success);
    background: oklch(0.620 0.080 160 / 0.10);
    border-color: oklch(0.620 0.080 160 / 0.30);
  }
  .chip-error {
    color: var(--ac-danger);
    background: oklch(0.550 0.180 28 / 0.10);
    border-color: oklch(0.550 0.180 28 / 0.32);
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
    background: oklch(0.550 0.180 28 / 0.10);
    border: 1px solid oklch(0.550 0.180 28 / 0.32);
  }
  .error-icon {
    color: var(--ac-danger);
    margin-top: 1px;
    flex-shrink: 0;
  }
  .error-text {
    min-width: 0;
    flex: 1;
    font-size: 12.5px;
    color: var(--ac-ink-2);
    line-height: 1.45;
  }
  .error-message {
    color: var(--ac-danger);
  }
  .retry {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--ac-accent);
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
    border: 1px solid var(--ac-edge);
    background: var(--ac-paper-3);
    transition: background 180ms ease, opacity 180ms ease, border-color 180ms ease;
  }
  .switch:disabled {
    cursor: default;
    opacity: 0.6;
  }
  .switch.on {
    background: var(--ac-ink);
    border-color: transparent;
  }
  .switch-knob {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--ac-paper);
    box-shadow: 0 1px 2px oklch(0 0 0 / 0.15);
    transition: left 180ms ease;
  }
  .switch-knob.on {
    left: 18px;
  }
</style>
