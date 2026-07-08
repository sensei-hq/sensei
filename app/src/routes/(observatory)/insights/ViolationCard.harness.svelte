<script lang="ts">
  import ViolationCard from './ViolationCard.svelte';
  import type { ViolationCardVm } from './insights-board.svelte.js';

  let { memory }: { memory: ViolationCardVm } = $props();
</script>

<ViolationCard {memory} />
