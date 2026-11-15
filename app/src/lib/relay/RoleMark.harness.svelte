<script lang="ts">
  // Test wrapper: provides a glyph mode through context, as each app's root
  // layout does, and renders RoleMark with plain props.
  import { setGlyphMode } from '$lib/glyphs';
  import type { RelayRole } from '$lib/relay-api.js';
  import type { RoleStatus } from './primitives.js';
  import RoleMark from './RoleMark.svelte';

  let {
    glyphs = 'kanji',
    role,
    status = 'active',
    size = 24,
  }: { glyphs?: 'kanji' | 'icon'; role: RelayRole; status?: RoleStatus; size?: 24 | 30 } = $props();

  setGlyphMode(() => glyphs);
</script>

<RoleMark {role} {status} {size} />
