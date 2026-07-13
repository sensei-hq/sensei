<script lang="ts">
  // Test wrapper — drives ProjectGlyph with a resolved ProjectIcon so the spec
  // can exercise the image render and its kanji load-error fallback.
  import ProjectGlyph from './ProjectGlyph.svelte';
  import type { ProjectIcon } from './buckets.js';

  let { icon }: { icon: ProjectIcon } = $props();
</script>

<ProjectGlyph {icon} />
