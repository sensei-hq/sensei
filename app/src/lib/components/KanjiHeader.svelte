<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    kanji: string;
    eyebrow: string;
    title: Snippet;
    right?: Snippet;
  }
  let { kanji, eyebrow, title, right }: Props = $props();
</script>

<section class="flex items-start gap-3">
  <span
    data-component="kanji-header-kanji"
    class="font-kanji text-accent text-2xl leading-none shrink-0"
  >{kanji}</span>

  <div class="flex-1 min-w-0">
    <div
      data-component="kanji-header-eyebrow"
      class="text-xs tracking-wide uppercase text-ink-mute font-medium leading-none"
    >{eyebrow}</div>
    <div
      data-component="kanji-header-title"
      class="font-display text-lg font-normal text-ink leading-snug mt-1"
    >{@render title()}</div>
  </div>

  {#if right}
    <div class="shrink-0">{@render right()}</div>
  {/if}
</section>
