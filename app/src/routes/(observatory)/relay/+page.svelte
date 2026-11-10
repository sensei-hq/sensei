<script lang="ts">
  // The main window's Relay (D90): every crew, with the repository/workspace
  // filters in the URL so a narrowed view is a link the person can keep.
  import { appState } from '$lib/appstate.svelte.js';
  import { relayApi } from '$lib/relay-api.js';
  import { RelayState } from '$lib/relay/relay-state.svelte.js';
  import { crewSizeContext } from '$lib/relay/relay-format.js';
  import RelayScreen from '$lib/relay/RelayScreen.svelte';

  let { data } = $props();

  const state = $derived.by(() => new RelayState(data, relayApi(appState.port), data.filter));

  // The crew's size, not its count: there is always one crew (#394).
  const context = $derived(crewSizeContext(state.now, state.nowError));
</script>

<RelayScreen {state} viewId="observatory.relay" base="/relay" {context} filtered />
