<script lang="ts">
  // A Now section's heading (COMPONENTS §2): 12px, .12em, uppercase, ink-mute, 500;
  // an optional mark in its tone; "Label · N" when it counts something.
  let { label, count, mark }: { label: string; count?: number; mark?: { glyph: string; tone: string } } = $props();
</script>

<h3
  data-section-heading
  class="m-0 flex items-center gap-1.5 text-xs font-medium uppercase tracking-[0.12em] text-ink-mute"
>
  {#if mark}<span aria-hidden="true" class="font-kanji {mark.tone}">{mark.glyph}</span>{/if}
  {label}{#if count !== undefined}&nbsp;·&nbsp;{count}{/if}
</h3>
