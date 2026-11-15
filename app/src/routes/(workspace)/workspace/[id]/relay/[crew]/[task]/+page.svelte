<script lang="ts">
  // The workspace window's task view (D90). Same screen as the main window's; what
  // differs is the view id and where the back link goes.
  import { appState } from '$lib/appstate.svelte.js';
  import { relayApi } from '$lib/relay-api.js';
  import { RelayTaskState } from '$lib/relay/relay-task-state.svelte.js';
  import { subscribeTaskStream } from '$lib/relay/relay-task-stream.js';
  import { crewTail } from '$lib/relay/relay-format.js';
  import RelayTaskScreen from '$lib/relay/RelayTaskScreen.svelte';

  let { data } = $props();

  /** Kept by the pair it was read for, so a navigation is a new task. */
  const state = $derived.by(
    () => new RelayTaskState(data, relayApi(appState.port), data.crewId, data.taskId),
  );

  // The route owns the socket (F8): task notifications re-read, steps apply.
  // Param-keyed, so navigating crew/task resubscribes with the new state.
  $effect(() => subscribeTaskStream(appState.port, state, data.taskId));
</script>

<RelayTaskScreen
  {state}
  viewId="workspace.relay.task"
  backHref={`/workspace/${data.workspaceId}/relay`}
  context={`crew ${crewTail(data.crewId)}`}
/>
