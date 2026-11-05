<script lang="ts">
  // The wait: pulsing skeleton rows on `bg-paper-mute`, busy, with words for a
  // screen reader. The animation is opacity only (no colour), and stops when the
  // reader asks for reduced motion.
  let {
    rows = 4,
    label = 'Loading…',
    ...rest
  }: { rows?: number; label?: string; [attr: `data-${string}`]: unknown } = $props();
  const WIDTHS = ['w-3/4', 'w-full', 'w-3/5', 'w-5/6'];
</script>

<div {...rest} data-component="loading" class="flex flex-col gap-3 py-10 px-6" aria-busy="true" aria-live="polite">
  {#each { length: rows } as _, i (i)}
    <div data-part="row" class="skeleton-bar h-4 rounded bg-paper-mute {WIDTHS[i % WIDTHS.length]}"></div>
  {/each}
  <span class="sr-only">{label}</span>
</div>

<style>
  .skeleton-bar {
    animation: loading-pulse 1.4s ease-in-out infinite;
  }
  @keyframes loading-pulse {
    0%, 100% { opacity: 0.55; }
    50% { opacity: 0.9; }
  }
  @media (prefers-reduced-motion: reduce) {
    .skeleton-bar { animation: none; }
  }
</style>
