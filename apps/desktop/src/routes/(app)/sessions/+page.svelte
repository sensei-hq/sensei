<script lang="ts">
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  onMount(() => {
    const activeSolution = localStorage.getItem('sensei:active_solution');
    goto(activeSolution ? `/s/${activeSolution}/sessions` : '/all', { replaceState: true });
  });
</script>
