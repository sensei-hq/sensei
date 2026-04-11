<script lang="ts">
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';

  onMount(() => {
    const done = localStorage.getItem('sensei:setup_complete');
    if (!done) { goto('/setup', { replaceState: true }); return; }
    const activeSolution = localStorage.getItem('sensei:active_solution');
    goto(activeSolution ? `/s/${activeSolution}` : '/all', { replaceState: true });
  });
</script>
