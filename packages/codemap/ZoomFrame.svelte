<script lang="ts">
  // A diagram with rokkit's zoom over it (M17): − / % / +, the % going back to
  // fit. The drawing reads the zoom and scales itself.
  import type { Snippet } from 'svelte';
  import { ZoomControl } from '@rokkit/graph';

  let { zoom, onzoom, children }: { zoom: number; onzoom: (z: number) => void; children: Snippet } = $props();
</script>

<div class="relative">
  <div class="absolute top-2 right-2 z-10 rounded bg-paper/90 shadow" data-testid="codemap-zoom">
    <ZoomControl {zoom} onchange={onzoom} />
  </div>
  {@render children()}
</div>
