<script lang="ts">
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';

  // Fallback: hooks.client.ts reroute handles this for all subsequent loads.
  // This fires only on the very first launch before localStorage is populated.
  onMount(() => {
    goto('/health', { replaceState: true });
  });
</script>
