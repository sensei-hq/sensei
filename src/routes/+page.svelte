<script lang="ts">
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { appState } from '$lib/appstate.svelte.js';
  import { runBootstrap } from '$lib/bootstrap.js';

  onMount(async () => {
    // Run sidecar detection first — daemon may not be running
    const result = await runBootstrap();

    if (!result.ready) {
      goto('/health', { replaceState: true });
      return;
    }

    // Daemon is running (bootstrap passed) — safe to load config
    await appState.load();

    if (appState.setupComplete) {
      goto('/observatory', { replaceState: true });
    } else {
      goto('/setup/welcome', { replaceState: true });
    }
  });
</script>
