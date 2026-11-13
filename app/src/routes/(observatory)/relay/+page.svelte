<script lang="ts">
  // The main window's Relay (D90): every crew, with the repository/workspace
  // filters in the URL so a narrowed view is a link the person can keep.
  import { appState } from '$lib/appstate.svelte.js';
  import { RelayState, relaySseSource } from '@sensei/state';
  import { relayApi, relayStream } from '$lib/relay-api.js';
  import RelayScreen from '$lib/relay/RelayScreen.svelte';

  let { data } = $props();

  const state = $derived.by(
    () => new RelayState(relaySseSource(relayApi(appState.port), () => relayStream(appState.port)), data.filter, data),
  );

  // The state listens through its source (#421): a row that arrives changes that row,
  // an id-only event reads again; a navigation connects the new state, closing the old.
  $effect(() => state.connect());

  // The crew's size, not its count: there is always one crew (#394).
  // The context names the place and the run (CONTRACT §1a); the crew's size is the
  // Crew button's badge. The main window shows every workspace's crews.
  const where = 'all workspaces';
</script>

<RelayScreen {state} viewId="observatory.relay" base="/relay" {where} filtered />
