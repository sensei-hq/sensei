<script lang="ts">
  // Test wrapper for the shared family's smaller parts (EmptyState, Banner,
  // Loading): provides a glyph mode through context, as each app's root does.
  import { EmptyState, Banner, Loading, setGlyphMode } from '@sensei/ui';

  let {
    part,
    glyphs = 'kanji',
    body,
    ...props
  }: { part: 'empty' | 'banner' | 'loading'; glyphs?: 'kanji' | 'icon'; body?: string; [k: string]: unknown } = $props();
  setGlyphMode(() => glyphs);
</script>

{#if part === 'empty'}
  {#if body}<EmptyState {...(props as any)}>{body}</EmptyState>{:else}<EmptyState {...(props as any)} />{/if}
{:else if part === 'banner'}
  {#if body}<Banner {...(props as any)}>{body}</Banner>{:else}<Banner {...(props as any)} />{/if}
{:else}
  <Loading {...(props as any)} />
{/if}
