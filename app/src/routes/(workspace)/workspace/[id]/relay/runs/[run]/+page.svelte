<script lang="ts">
  // The workspace window's past run (#424). Same screen as the main window's; what
  // differs is the view id and the Relay it goes back to.
  import RelayRunScreen from '$lib/relay/RelayRunScreen.svelte';

  let { data } = $props();
</script>

<RelayRunScreen
  detail={data.detail}
  error={data.error}
  viewId="workspace.relay.run"
  base={`/workspace/${data.workspaceId}/relay`}
/>
