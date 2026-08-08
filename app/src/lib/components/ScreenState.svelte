<script lang="ts">
  import type { Snippet } from 'svelte';
  import EmptyState from './EmptyState.svelte';

  /**
   * The shared screen-state scaffold (mockup-drift-audit F8). One place that
   * renders the four states every data screen has — loading skeleton, empty,
   * error-with-Retry, and ready — so a fetch FAILURE is always visibly distinct
   * from an honest-empty result (the no-fabrication guarantee) rather than both
   * rendering the same blank. Loaders return an `error` on failure and the
   * screen passes `status="error"` here instead of fabricating an empty payload.
   *
   * `empty` renders the standard EmptyState from kanji/title/description; a screen
   * whose empty needs a bespoke call-to-action keeps that inside `ready` children
   * and only uses this for loading/error.
   */
  let {
    status,
    error = null,
    onretry,
    kanji = '',
    title = '',
    description = '',
    children,
  }: {
    status: 'loading' | 'error' | 'empty' | 'ready';
    error?: string | null;
    onretry?: () => void;
    kanji?: string;
    title?: string;
    description?: string;
    children?: Snippet;
  } = $props();
</script>

{#if status === 'loading'}
  <div
    class="flex flex-col gap-3 py-10 px-6"
    data-screen-state="loading"
    aria-busy="true"
    aria-live="polite"
  >
    {#each [72, 94, 60, 86] as w, i (i)}
      <div class="skeleton-bar h-4 rounded bg-paper-mute" style="width: {w}%"></div>
    {/each}
    <span class="sr-only">Loading…</span>
  </div>
{:else if status === 'error'}
  <div
    class="flex flex-col items-center text-center py-20 gap-4"
    data-screen-state="error"
    role="alert"
  >
    <span class="kanji text-4xl text-danger opacity-40" aria-hidden="true">難</span>
    <p class="display text-xl font-normal m-0 text-ink">Couldn't load this</p>
    <p class="text-sm text-ink-soft max-w-[420px] leading-normal m-0">
      {error ?? 'sensei couldn’t be reached. Check that the daemon is running, then retry.'}
    </p>
    {#if onretry}
      <button
        type="button"
        class="zs-btn zs-btn-secondary"
        data-action="retry"
        onclick={onretry}
      >Retry</button>
    {/if}
  </div>
{:else if status === 'empty'}
  <EmptyState {kanji} {title} {description} />
{:else}
  {@render children?.()}
{/if}

<style>
  /* Color-free pulse — the token-driven bg-paper-mute supplies the colour; this
     only animates opacity, so it respects both light/dark without any literal. */
  .skeleton-bar {
    animation: screenstate-pulse 1.4s ease-in-out infinite;
  }
  @keyframes screenstate-pulse {
    0%, 100% { opacity: 0.55; }
    50% { opacity: 0.9; }
  }
  @media (prefers-reduced-motion: reduce) {
    .skeleton-bar { animation: none; }
  }
</style>
