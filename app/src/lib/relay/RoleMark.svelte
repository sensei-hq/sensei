<script lang="ts">
  // A lane's role, as the reader's glyph mode draws marks: 指 for the coordinator
  // (the hand that points) and 手 for a contributor (the hand that works), or the
  // vocabulary's icon for the kanji (#133). At 24px it marks a lane and carries
  // the role; at 30px it marks a crew member and carries the session's status —
  // in the fill, the border, the glyph and the title, never colour alone
  // (COMPONENTS.md §3a).
  import type { RelayRole } from '$lib/relay-api.js';
  import { KANJI_ICONS } from '$lib/glyph-vocabulary';
  import { useGlyphMode } from '$lib/glyphs';
  import { roleMarkClass, type RoleStatus } from './primitives.js';

  let {
    role,
    status = 'active',
    size = 24,
  }: { role: RelayRole; status?: RoleStatus; size?: 24 | 30 } = $props();

  const mode = useGlyphMode();
  const kanji = $derived(role === 'coordinator' ? '指' : '手');
  const mark = $derived(roleMarkClass(role, status, size));
</script>

<span
  data-role-mark
  data-role={role}
  data-size={size}
  data-status={size === 30 ? status : undefined}
  title={mark.title}
  class={mark.frame}
>
  {#if mode() === 'icon'}
    <span data-lane-role class={[KANJI_ICONS[kanji], mark.glyph]} aria-hidden="true"></span>
  {:else}
    <span data-lane-role class={['font-kanji leading-none', mark.glyph]} aria-hidden="true">{kanji}</span>
  {/if}
</span>
