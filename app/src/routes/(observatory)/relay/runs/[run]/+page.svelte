<script lang="ts">
  // The main window's past run (#424): the shared screen under the main window's Relay.
  import RelayRunScreen from '$lib/relay/RelayRunScreen.svelte';

  let { data } = $props();
</script>

<RelayRunScreen detail={data.detail} error={data.error} viewId="observatory.relay.run" base="/relay" />
