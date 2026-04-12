<script lang="ts">
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  onMount(() => {
    const active = localStorage.getItem('sensei:active_solution');
    goto(active ? `/s/${active}/indexer` : '/all', { replaceState: true });
  });
</script>
