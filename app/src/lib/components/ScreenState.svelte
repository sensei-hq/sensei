<script lang="ts">
  import type { Snippet } from 'svelte';
  import { Notice } from '@sensei/ui';
  import { screenErrorNotice, type ScreenNotice } from './screen-state-notice.js';
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
    notice,
    children,
  }: {
    status: 'loading' | 'error' | 'empty' | 'ready';
    /** What failed and what to do, from the screen's state module
     *  (`noticeFromApiError`). Without it the error branch says it generically. */
    notice?: ScreenNotice;
    error?: string | null;
    onretry?: () => void;
    kanji?: string;
    title?: string;
    description?: string;
    children?: Snippet;
  } = $props();

  const view = $derived(screenErrorNotice(error, notice));
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
  <!-- The shared Notice (docs/architecture/notice.md): a screen that failed reads
       the same in sensei and the dōjō. The copy is the caller's `notice` when its
       state module has one, else built from `error` here. -->
  <Notice
    size="screen"
    tone={view.tone}
    eyebrow={view.eyebrow}
    title={view.title}
    detail={view.detail}
    remedy={view.remedy}
    code={view.code}
    primary={onretry ? { label: 'Retry', onclick: onretry, name: 'retry' } : undefined}
    data-screen-state="error"
  />
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
