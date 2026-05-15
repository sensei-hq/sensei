<script lang="ts">
  import type { Snippet } from 'svelte';

  type Tone = 'muted' | 'ink';

  let {
    tone = 'muted',
    children,
  }: { tone?: Tone; children: Snippet } = $props();

  const toneClass = $derived(tone === 'ink' ? 'text-ink-z9' : 'text-ink-z6');
</script>

<span
  data-component="eyebrow"
  class="text-xs tracking-wide uppercase {toneClass}"
>
  {@render children()}
</span>
