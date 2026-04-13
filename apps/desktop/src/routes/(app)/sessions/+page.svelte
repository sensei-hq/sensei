<script lang="ts">
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { getActiveSolutionId } from '$lib/appstate.svelte.js';
  onMount(() => {
    const activeSolution = getActiveSolutionId();
    goto(activeSolution ? `/s/${activeSolution}/sessions` : '/all', { replaceState: true });
  });
</script>
