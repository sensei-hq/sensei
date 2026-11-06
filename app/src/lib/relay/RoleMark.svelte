<script lang="ts">
  // A lane's role, as the reader's glyph mode draws marks: 指 for the coordinator
  // (the hand that points) and 手 for a contributor (the hand that works), or the
  // vocabulary's icon for the kanji (#133).
  import type { RelayRole } from '$lib/relay-api.js';
  import { KANJI_ICONS } from '$lib/glyph-vocabulary';
  import { useGlyphMode } from '$lib/glyphs';

  let { role }: { role: RelayRole } = $props();

  const mode = useGlyphMode();
  const kanji = $derived(role === 'coordinator' ? '指' : '手');
</script>

{#if mode() === 'icon'}
  <span data-lane-role class={[KANJI_ICONS[kanji], 'shrink-0']} aria-hidden="true"></span>
{:else}
  <span data-lane-role class="font-kanji leading-none shrink-0" aria-hidden="true">{kanji}</span>
{/if}
