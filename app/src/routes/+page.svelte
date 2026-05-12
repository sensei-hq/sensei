<script lang="ts">
  // Root page — reroute in hooks.client.ts handles all navigation:
  //   health not ready → /health
  //   setup not done   → /setup/welcome
  //   both gates pass  → /observatory
  //
  // This page only renders if reroute didn't fire (e.g. direct browser access).
  // In that case, redirect to /observatory and let the guard cascade handle it.
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';

  onMount(() => {
    goto('/observatory', { replaceState: true });
  });
</script>

<div class="flex items-center justify-center h-screen bg-surface-z1 text-surface-z6">
  <span class="kanji text-2xl text-primary-z5 opacity-40">先</span>
</div>
