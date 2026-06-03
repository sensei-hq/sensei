<script lang="ts">
  import GateRow from './GateRow.svelte';
  import type { Component } from '$lib/health-types.js';

  let { gate, numeral }: { gate: Component; numeral: string } = $props();
</script>

<GateRow {gate} {numeral} />
