<script lang="ts">
  import type { Snippet } from 'svelte';

  /** Color utility class (e.g. `text-ink-mute`, `text-ink`, `text-accent`).
   *  Defaults to ink-mute — eyebrows are secondary labels above a headline. */
  let {
    tone = 'text-ink-mute',
    children,
  }: { tone?: string; children: Snippet } = $props();
</script>

<span
  data-component="eyebrow"
  class="text-xs tracking-wide uppercase {tone}"
>
  {@render children()}
</span>
