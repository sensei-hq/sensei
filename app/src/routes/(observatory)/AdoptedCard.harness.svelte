<script lang="ts">
  import AdoptedCard from './AdoptedCard.svelte';
  import type { ObservatoryAdopted } from '$lib/types.js';

  let { item }: { item: ObservatoryAdopted } = $props();
</script>

<AdoptedCard {item} />
