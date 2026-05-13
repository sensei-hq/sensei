<script lang="ts">
  import type { Component, HealthStatus } from '$lib/health-types.js';

  interface Props {
    packageManager: Component;
    status: HealthStatus;
    components: Component[];
    onEnter?: () => void;
  }
  let { packageManager, status, components, onEnter }: Props = $props();

  const activeIdx = $derived.by(() => {
    const i = components.findIndex(
      (c) => c.status === 'installing' || c.status === 'checking'
    );
    if (i >= 0) return i;
    return components.filter((c) => c.status === 'ready').length - 1;
  });
  const total = $derived(components.length);
  const activeLabel = $derived(
    components[Math.min(activeIdx, total - 1)]?.label ?? ''
  );
  const activeCount = $derived(Math.min(activeIdx + 1, total));
</script>

<section class="border border-surface-z2 rounded-xl bg-surface-z1 p-6.5">
  <div class="flex items-center gap-4.5">
    <div class="w-14 h-14 rounded-full border-[1.5px] flex items-center justify-center shrink-0"
         class:border-success-z5={status === 'ok'}
         class:border-primary-z5={status === 'needs-action'}
         class:border-surface-z3={status === 'checking' || status === 'resolving'}>
      {#if status === 'ok'}
        <span class="text-2xl text-success-z5 leading-none">✓</span>
      {:else if status === 'needs-action'}
        <span class="kanji text-xl text-primary-z5">?</span>
      {:else}
        <span class="spinner-ring"></span>
      {/if}
    </div>

    <div class="flex-1 min-w-0">
      <div class="flex items-baseline gap-2.5 mb-1">
        <span class="display text-prose font-medium text-ink-z1">{packageManager.label}</span>
        {#if packageManager.note}
          <span class="mono text-2xs text-ink-z5">{packageManager.note}</span>
        {/if}
      </div>
      <div class="text-sm text-ink-z3 leading-snug">
        {#if status === 'ok'}
          Detected. All dependencies installed.
        {:else if status === 'needs-action'}
          Couldn't finish automatically. Run the script below.
        {:else if status === 'resolving'}
          Detected. Installing
          <span class="text-ink-z1">{activeLabel}</span>
          <span class="mono text-2xs text-ink-z5 ml-2">({activeCount}/{total})</span>
        {:else}
          Checking system…
        {/if}
      </div>
    </div>

    {#if status === 'ok'}
      <button data-action="enter" class="btn-solid shrink-0" onclick={onEnter}>Enter</button>
    {/if}
  </div>
</section>

<style>
  .spinner-ring {
    display: block;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    border: 2px solid oklch(var(--color-surface-z3) / 1);
    border-top-color: transparent;
    animation: spin 0.9s linear infinite;
  }
  @keyframes spin { to { transform: rotate(360deg); } }
</style>
