<script lang="ts">
  // A diagram with rokkit's zoom over it (M17): − / % / +, the % going back to
  // fit. The drawing scrolls inside a frame of fixed height, and rokkit's overlay
  // strip holds the zoom at the frame's bottom right, in view however far it
  // scrolls. The drawing reads the zoom and scales itself.
  import type { Snippet } from 'svelte';
  import { ZoomControl } from '@rokkit/graph';

  let { zoom, onzoom, children }: { zoom: number; onzoom: (z: number) => void; children: Snippet } = $props();
</script>

<div class="relative flex-1 min-h-0 flex flex-col" data-testid="codemap-zoom-frame">
  <!-- A scrollable region must take focus for keyboard users (axe scrollable-region-focusable). -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <div class="flex-1 min-h-0 overflow-auto pb-12" tabindex="0" role="region" aria-label="Diagram, scrollable">
    {@render children()}
  </div>
  <div data-graph-diagram-overlay data-testid="codemap-zoom">
    <ZoomControl {zoom} onchange={onzoom} />
  </div>
</div>
