<script lang="ts">
  import AtlasInspector from './AtlasInspector.svelte';
  import type {
    InspectorCommunity,
    InspectorSymbol,
    InspectorOverview,
  } from './atlas-graph.svelte.js';

  let {
    community = null,
    symbol = null,
    dependsOn = [],
    usedBy = [],
    overview,
  }: {
    community?: InspectorCommunity | null;
    symbol?: InspectorSymbol | null;
    dependsOn?: string[];
    usedBy?: string[];
    overview: InspectorOverview;
  } = $props();
</script>

<AtlasInspector {community} {symbol} {dependsOn} {usedBy} {overview} />
