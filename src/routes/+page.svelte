<script lang="ts">
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';

  // '/' is exempt from the health gate in hooks.client.ts reroute (WKWebView
  // TDZ workaround). This onMount redirect fires on every cold start so that
  // SvelteKit can fully initialize before the heavier /health modules load.
  onMount(() => {
    goto('/health', { replaceState: true });
  });
</script>
