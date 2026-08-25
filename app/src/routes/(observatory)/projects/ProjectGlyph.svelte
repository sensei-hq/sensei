<script lang="ts">
  // The project identity mark shared by the card and the row. Renders the
  // inferred repo logo as a small image when the icon resolves to one, and
  // falls back to the kanji glyph on any load error (a broken image is worse
  // than a glyph). The image-vs-kanji decision itself is the pure `projectIcon`
  // helper in buckets.ts; this component only owns the load-error fallback.
  import type { ProjectIcon } from './buckets.js';

  let { icon }: { icon: ProjectIcon } = $props();

  // The src that failed to load, if any. Keyed on the src (not a bare boolean)
  // so a reused component instance retries when the project — and thus the
  // src — changes, rather than staying stuck on a prior failure.
  let erroredSrc = $state<string | null>(null);
</script>

{#if icon.kind === 'image' && erroredSrc !== icon.src}
  {@const src = icon.src}
  <img
    {src}
    alt=""
    onerror={() => (erroredSrc = src)}
    class="w-[18px] h-[18px] object-contain rounded-sm shrink-0"
  />
{:else}
  <span class="kanji text-accent text-lg leading-none shrink-0">{icon.glyph}</span>
{/if}
