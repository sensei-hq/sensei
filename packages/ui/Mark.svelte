<script lang="ts">
  // A mark that follows the glyph mode (#133): the kanji in kanji mode, the
  // vocabulary's icon in icon mode (a kanji with no icon stays a kanji). Shared by
  // EmptyState and Banner; Notice draws its own the same way.
  import { useGlyphMode } from './glyph-mode.js';
  import { noticeMark } from './notice.js';

  let { char, tone = 'normal', class: cls = '' }: { char?: string; tone?: string; class?: string } = $props();
  const mode = useGlyphMode();
  const glyph = $derived(noticeMark(tone, mode(), { char }));
</script>

{#if glyph.mode === 'icon'}
  <span data-part="mark" data-glyph="icon" class={[glyph.icon, cls, 'shrink-0']} aria-hidden="true"></span>
{:else}
  <span data-part="mark" data-glyph="kanji" class={['font-kanji leading-none', cls, 'shrink-0']} aria-hidden="true">{glyph.char}</span>
{/if}
