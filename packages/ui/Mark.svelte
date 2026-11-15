<script lang="ts">
  // A mark that follows the glyph mode (#133): the kanji in kanji mode, the
  // vocabulary's icon in icon mode (a kanji with no icon stays a kanji). The one
  // mark of the family: Notice, EmptyState and Banner all draw it.
  import { useGlyphMode } from './glyph-mode.js';
  import { noticeMark } from './notice.js';

  let {
    char,
    icon,
    tone = 'normal',
    mode: modeProp,
    class: cls = '',
  }: {
    char?: string;
    /** The icon-mode mark, when not the vocabulary's icon for the kanji. */
    icon?: string;
    tone?: string;
    /** Overrides the provided glyph mode. */
    mode?: 'kanji' | 'icon';
    class?: string;
  } = $props();
  const provided = useGlyphMode();
  const glyph = $derived(noticeMark(tone, modeProp ?? provided(), { char, icon }));
</script>

{#if glyph.mode === 'icon'}
  <span data-part="mark" data-glyph="icon" class={[glyph.icon, cls, 'shrink-0']} aria-hidden="true"></span>
{:else}
  <span data-part="mark" data-glyph="kanji" class={['font-kanji leading-none', cls, 'shrink-0']} aria-hidden="true">{glyph.char}</span>
{/if}
