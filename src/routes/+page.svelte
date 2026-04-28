<script lang="ts">
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { isTestMode } from '$lib/mock-data.js';
  import { appState } from '$lib/appstate.svelte.js';
  import { runBootstrap } from '$lib/bootstrap.js';

  onMount(async () => {
    // Test mode: skip daemon calls, go straight to health page with mock data
    if (isTestMode()) {
      goto('/health', { replaceState: true });
      return;
    }

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
