<script lang="ts">
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  onMount(() => goto(`/project/${$page.params.id}/overview`, { replaceState: true }));
</script>
