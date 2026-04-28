<script lang="ts">
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { loadAppState, isSetupComplete } from '$lib/appstate.svelte.js';
  import { runBootstrap } from '$lib/bootstrap.js';

  onMount(async () => {
    await loadAppState();

    const result = await runBootstrap();

    if (!result.ready) {
      goto('/health', { replaceState: true });
      return;
    }

    if (isSetupComplete()) {
      goto('/observatory', { replaceState: true });
    } else {
      goto('/config', { replaceState: true });
    }
  });
</script>
