<script lang="ts">
  // The main window's Relay (D90): every crew, with the repository/workspace
  // filters in the URL so a narrowed view is a link the person can keep.
  import { appState } from '$lib/appstate.svelte.js';
  import { relayApi } from '$lib/relay-api.js';
  import { RelayState } from '$lib/relay/relay-state.svelte.js';
  import RelayScreen from '$lib/relay/RelayScreen.svelte';

  let { data } = $props();

  const state = $derived.by(() => new RelayState(data, relayApi(appState.port), data.filter));

  const context = $derived.by(() => {
    const crews = state.now?.crews.length ?? 0;
    return `${crews} ${crews === 1 ? 'crew' : 'crews'}`;
  });
</script>

<RelayScreen {state} viewId="observatory.relay" base="/relay" {context} filtered />
