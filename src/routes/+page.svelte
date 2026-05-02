<script lang="ts">
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';

  // Immediately send to the health/bootstrap page — it handles detection,
  // shows status while gates are checking, and auto-advances to
  // /setup/welcome or /observatory once everything is ready.
  onMount(() => {
    goto('/health', { replaceState: true });
  });
</script>
