<script lang="ts">
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';

  onMount(() => {
    const done = localStorage.getItem('sensei:setup_complete');
    goto(done ? '/projects' : '/setup', { replaceState: true });
  });
</script>
