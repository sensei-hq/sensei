<script lang="ts">
  // The main window's task view (D90): the same screen, on observatory.relay.task,
  // with the back link to the main Relay.
  import { appState } from '$lib/appstate.svelte.js';
  import { relayApi, relayStream } from '$lib/relay-api.js';
  import { RelayTaskState, relayTaskSseSource } from '@sensei/state';
  import { crewTail } from '$lib/relay/relay-format.js';
  import RelayTaskScreen from '$lib/relay/RelayTaskScreen.svelte';

  let { data } = $props();

  const state = $derived.by(
    () =>
      new RelayTaskState(
        data,
        relayTaskSseSource(relayApi(appState.port), () => relayStream(appState.port, data.crewId)),
        data.crewId,
        data.taskId,
      ),
  );

  // The state listens through its source (#414), to its crew's stream only (#418):
  // steps apply, task notifications re-read. Param-keyed, so navigating crew/task
  // connects the new state and closes the old.
  $effect(() => state.connect());
</script>

<RelayTaskScreen
  {state}
  viewId="observatory.relay.task"
  backHref="/relay"
  context={`crew ${crewTail(data.crewId)}`}
/>
