<script lang="ts">
  import type { Snippet } from 'svelte';

  /** Color utility class (e.g. `text-ink-soft`, `text-ink`, `text-accent`).
   *  Defaults to ink-soft — eyebrows are secondary labels above a headline. */
  let {
    tone = 'text-ink-soft',
    children,
  }: { tone?: string; children: Snippet } = $props();
</script>

<span
  data-component="eyebrow"
  class="text-xs tracking-[0.18em] uppercase {tone}"
>
  {@render children()}
</span>
