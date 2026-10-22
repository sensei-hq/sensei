<script lang="ts">
  // Test wrapper — drives WorkspaceGlyph with a resolved WorkspaceIcon so the spec
  // can exercise the image render and its kanji load-error fallback.
  import WorkspaceGlyph from './WorkspaceGlyph.svelte';
  import type { WorkspaceIcon } from './buckets.js';

  let { icon }: { icon: WorkspaceIcon } = $props();
</script>

<WorkspaceGlyph {icon} />
