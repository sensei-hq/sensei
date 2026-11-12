<script lang="ts">
  // The workspace window's Relay (D90). The loader reads; the screen draws. The
  // layout's workspace name is the first half of the context line (CONTRACT §1a).
  import { appState } from '$lib/appstate.svelte.js';
  import { relayApi } from '$lib/relay-api.js';
  import { RelayState } from '$lib/relay/relay-state.svelte.js';
  import { subscribeCrewStream } from '$lib/relay/relay-crew-stream.js';
  import { crewTail } from '$lib/relay/relay-format.js';
  import RelayScreen from '$lib/relay/RelayScreen.svelte';

  let { data } = $props();

  /** Re-created when the loader's answer changes, so a navigation never keeps the
   *  old crew or the old queue beside the new one. */
  const state = $derived.by(() => new RelayState(data, relayApi(appState.port), data.filter));

  // The route owns the socket (queue cutover): a run, lane or task event
  // re-reads the crew; a navigation resubscribes with the new state.
  $effect(() => subscribeCrewStream(appState.port, state));

  const workspaceName = $derived(
    (data as unknown as { workspace?: { name: string } }).workspace?.name ?? null,
  );
  const context = $derived(
    [workspaceName, state.crew ? `crew ${crewTail(state.crew.id)}` : null].filter(Boolean).join(' · '),
  );
</script>

<RelayScreen
  {state}
  viewId="workspace.relay"
  base={`/workspace/${data.workspaceId}/relay`}
  {context}
/>
