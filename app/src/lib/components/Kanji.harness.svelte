<script lang="ts">
  // Test wrapper: provides a glyph mode through context, as the root layout
  // does, and renders Kanji WITHOUT a mode prop.
  import Kanji from './Kanji.svelte';
  import { setGlyphMode, type GlyphMode } from '$lib/glyphs';

  let { char, icon, glyphs }: { char: string; icon?: string; glyphs: GlyphMode } = $props();
  setGlyphMode(() => glyphs);
</script>

<Kanji {char} {icon} />
