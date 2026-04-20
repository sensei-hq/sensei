<script lang="ts">
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { getPort, isSetupComplete, getActiveSolutionId } from '$lib/appstate.svelte.js';
  import { senseiApi } from '$lib/api.js';

  onMount(async () => {
    // Check if daemon has projects (skip setup if data exists)
    const api = senseiApi(getPort());
    const projects = await api.getProjects();

    if (projects.length === 0 && !isSetupComplete()) {
      goto('/setup', { replaceState: true });
      return;
    }

    goto('/overview', { replaceState: true });
  });
</script>
