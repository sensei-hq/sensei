<script lang="ts">
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { appState } from '$lib/appstate.svelte.js';
  import { runBootstrap } from '$lib/bootstrap.js';

  onMount(async () => {
    await appState.load();

    const result = await runBootstrap();

    if (!result.ready) {
      goto('/health', { replaceState: true });
      return;
    }

    if (appState.setupComplete) {
      goto('/observatory', { replaceState: true });
    } else {
      goto('/setup/welcome', { replaceState: true });
    }
  });
</script>
