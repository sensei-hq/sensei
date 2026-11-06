<script lang="ts">
  // The main window's task view (D90): the same screen, on observatory.relay.task,
  // with the back link to the main Relay.
  import { appState } from '$lib/appstate.svelte.js';
  import { relayApi } from '$lib/relay-api.js';
  import { RelayTaskState } from '$lib/relay/relay-task-state.svelte.js';
  import { crewTail } from '$lib/relay/relay-format.js';
  import RelayTaskScreen from '$lib/relay/RelayTaskScreen.svelte';

  let { data } = $props();

  const state = $derived.by(
    () => new RelayTaskState(data, relayApi(appState.port), data.crewId, data.taskId),
  );
</script>

<RelayTaskScreen
  {state}
  viewId="observatory.relay.task"
  backHref="/relay"
  context={`crew ${crewTail(data.crewId)}`}
/>
