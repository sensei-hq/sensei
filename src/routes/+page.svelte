<script lang="ts">
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { loadAppState, getPort, isSetupComplete } from '$lib/appstate.svelte.js';
  import { senseiApi } from '$lib/api.js';

  onMount(async () => {
    await loadAppState();
    const api = senseiApi(getPort());

    try {
      const health = await api.getHealth();
      if (!health?.ok) {
        goto('/health', { replaceState: true });
        return;
      }
    } catch {
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
