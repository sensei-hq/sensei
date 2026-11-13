<script lang="ts">
  // The workspace window's Relay (D90). The loader reads; the screen draws. The
  // layout's workspace name is the first half of the context line (CONTRACT §1a).
  import { appState } from '$lib/appstate.svelte.js';
  import { RelayState, relaySseSource } from '@sensei/state';
  import { relayApi, relayStream } from '$lib/relay-api.js';
  import RelayScreen from '$lib/relay/RelayScreen.svelte';

  let { data } = $props();

  /** Re-created when the loader's answer changes, so a navigation never keeps the
   *  old crew or the old queue beside the new one. */
  const state = $derived.by(
    () => new RelayState(relaySseSource(relayApi(appState.port), () => relayStream(appState.port)), data.filter, data),
  );

  // The state listens through its source (#421): a row that arrives changes that row,
  // an id-only event reads again; a navigation connects the new state, closing the old.
  $effect(() => state.connect());

  // The context names the place and the run (CONTRACT §1a): the workspace by its
  // name, or by its window when the layout did not give one.
  const where = $derived(
    (data as unknown as { workspace?: { name: string } }).workspace?.name ?? 'this workspace',
  );
</script>

<RelayScreen
  {state}
  viewId="workspace.relay"
  base={`/workspace/${data.workspaceId}/relay`}
  {where}
/>
