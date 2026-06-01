<script lang="ts">
  import { brandGlyph } from '$lib/icons/brand-glyphs.js';

  let {
    id, name, size = 21,
  }: {
    /** Family id — keys into the bundled glyph registry. */
    id: string;
    /** Family display name — first letter used as the fallback glyph. */
    name: string;
    /** Square dimension in CSS pixels. Matches the mockup default (21px). */
    size?: number;
  } = $props();

  const glyph = $derived(brandGlyph(id));
  const letter = $derived((name?.[0] ?? '?').toUpperCase());
</script>

{#if glyph}
  <!-- SVG body is from @iconify-json/simple-icons (CC0-1.0) and embedded
       as a string at module load — no runtime fetch, safe under Tauri's
       offline-by-default network policy. -->
  <svg
    width={size}
    height={size}
    viewBox={glyph.viewBox}
    aria-hidden="true"
    style="display:block"
    >{@html glyph.body}</svg>
{:else}
  <span class="letter" style="font-size: {Math.round(size * 0.7)}px;">{letter}</span>
{/if}

<style>
  .letter {
    font-family: var(--font-display, system-ui);
    font-weight: 600;
    line-height: 1;
    color: currentColor;
  }
</style>
