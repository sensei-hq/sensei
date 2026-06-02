<script lang="ts">
  import type { Component, HealthStatus } from '$lib/health-types.js';
  import { Eyebrow, Kanji } from '$lib/components';

  interface Props {
    /** Overall daemon health status. */
    status: HealthStatus;
    /** All 6 gates (PM + 5 components). Used for the "X of 6 ready" line. */
    components: Component[];
  }
  let { status, components }: Props = $props();

  const isBusy = $derived(status === 'checking' || status === 'resolving');
  const total      = $derived(components.length);
  const readyCount = $derived(components.filter((c) => c.status === 'ready').length);

  const activeLabel = $derived.by(() => {
    const c = components.find((c) => c.status === 'installing' || c.status === 'checking');
    return c?.label ?? '';
  });

  const discBorderClass = $derived.by(() => {
    if (status === 'ok')           return 'border-success-z5';
    if (status === 'needs-action') return 'border-primary-z5';
    return 'border-primary-z5'; // checking / resolving — busy
  });
</script>

<section class="flex items-center gap-3">
  <div class="hero-disc {discBorderClass}">
    {#if status === 'ok'}
      <svg width="14" height="14" viewBox="0 0 10 10" fill="none">
        <path d="M2 5.2 L4.2 7.2 L8 3" stroke="currentColor"
              stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"
              class="text-success-z5"/>
      </svg>
    {:else if isBusy}
      <span data-hero-spinner class="hero-spinner"></span>
    {:else if status === 'needs-action'}
      <Kanji char="?" size="lg" />
    {/if}
  </div>

  <div class="min-w-0">
    <div class="flex items-baseline gap-1.5">
      <Kanji char="支" size="sm" />
      <Eyebrow>foundation</Eyebrow>
    </div>
    <div class="text-base text-surface-z7 leading-snug mt-0.5">
      {#if status === 'ok'}
        The foundation <span class="text-surface-z9 font-medium">holds.</span>
      {:else if status === 'needs-action'}
        One component <span class="text-surface-z9 font-medium">needs your hand.</span>
      {:else if status === 'resolving'}
        {#if activeLabel}
          <span class="text-surface-z9 font-medium">Installing</span>
          <span class="text-surface-z9">{activeLabel}</span>
          · {readyCount} of {total} ready.
        {:else}
          <span class="text-surface-z9 font-medium">Installing</span>
          · {readyCount} of {total} ready.
        {/if}
      {:else}
        <span class="text-surface-z9 font-medium">Checking</span> each component.
      {/if}
    </div>
  </div>
</section>

<style>
  .hero-disc {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    border-width: 2px;
    border-style: solid;
    background: oklch(var(--color-surface-z0) / 1);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }
  .hero-spinner {
    display: block;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    border: 2px solid oklch(var(--color-primary-z5) / 1);
    border-top-color: transparent;
    animation: hero-spin 0.9s linear infinite;
  }
  @keyframes hero-spin { to { transform: rotate(360deg); } }
</style>
