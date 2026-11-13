<script lang="ts">
  // The main window's Relay (D90): every crew, with the repository/workspace
  // filters in the URL so a narrowed view is a link the person can keep.
  import { appState } from '$lib/appstate.svelte.js';
  import { relayApi } from '$lib/relay-api.js';
  import { RelayState } from '$lib/relay/relay-state.svelte.js';
  import { subscribeCrewStream } from '$lib/relay/relay-crew-stream.js';
  import RelayScreen from '$lib/relay/RelayScreen.svelte';

  let { data } = $props();

  const state = $derived.by(() => new RelayState(data, relayApi(appState.port), data.filter));

  // The route owns the socket (queue cutover): a run, lane or task event
  // re-reads the crew; a navigation resubscribes with the new state.
  $effect(() => subscribeCrewStream(appState.port, state));

  // The crew's size, not its count: there is always one crew (#394).
  // The context names the place and the run (CONTRACT §1a); the crew's size is the
  // Crew button's badge. The main window shows every workspace's crews.
  const where = 'all workspaces';
</script>

<RelayScreen {state} viewId="observatory.relay" base="/relay" {where} filtered />
