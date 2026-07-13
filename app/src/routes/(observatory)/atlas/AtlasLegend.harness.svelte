<script lang="ts">
  import AtlasLegend from './AtlasLegend.svelte';
  import type { LegendItem } from './atlas-graph.svelte.js';

  let { items }: { items: LegendItem[] } = $props();
</script>

<AtlasLegend {items} />
