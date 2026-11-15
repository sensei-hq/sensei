<script lang="ts">
  // The shared eyebrow (packages/ui/Eyebrow.svelte) — kept at this path so the
  // app's imports and specs are unchanged.
  import type { Snippet } from 'svelte';
  import { Eyebrow } from '@sensei/ui';

  let { tone = 'text-ink-soft', children }: { tone?: string; children: Snippet } = $props();
</script>

<Eyebrow {tone}>{@render children()}</Eyebrow>
