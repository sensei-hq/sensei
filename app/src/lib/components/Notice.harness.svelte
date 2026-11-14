<script lang="ts">
  // Test wrapper: provides a glyph mode through context, as each app's root
  // layout does, and renders the shared Notice with plain props.
  import { Notice, setGlyphMode } from '@sensei/ui';

  let { glyphs = 'kanji', ...props }: { glyphs?: 'kanji' | 'icon'; [k: string]: unknown } = $props();
  setGlyphMode(() => glyphs);
</script>

<Notice {...(props as any)} />
