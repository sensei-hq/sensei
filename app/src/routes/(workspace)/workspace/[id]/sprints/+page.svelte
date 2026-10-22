<script lang="ts">
  // 期 Sprints — the workspace's sprints and tickets with their AI usage.
  // Client-side state: selecting a sprint or a ticket reads on demand.
  import { page } from '$app/state';
  import { senseiApi } from '$lib/api.js';
  import { appState } from '$lib/appstate.svelte.js';
  import SprintsScreen from './SprintsScreen.svelte';
  import { SprintsState } from './sprints-state.svelte.js';

  const workspaceId = $derived(page.params.id ?? '');
  // One screen state per workspace: switching workspace on this route starts fresh.
  const screen = $derived(new SprintsState(senseiApi(appState.port), workspaceId));
  $effect(() => {
    void screen.load();
  });
</script>

<SprintsScreen {screen} {workspaceId} />
