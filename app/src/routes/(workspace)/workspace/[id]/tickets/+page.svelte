<script lang="ts">
  // 札 Tickets — the workspace's tickets and their AI usage, filtered by sprint.
  // Client-side state: selecting a sprint or a ticket reads on demand.
  import { page } from '$app/state';
  import { senseiApi } from '$lib/api.js';
  import { appState } from '$lib/appstate.svelte.js';
  import TicketsScreen from './TicketsScreen.svelte';
  import { TicketsState } from './tickets-state.svelte.js';

  const workspaceId = $derived(page.params.id ?? '');
  // One screen state per workspace: switching workspace on this route starts fresh.
  const screen = $derived(new TicketsState(senseiApi(appState.port), workspaceId));
  $effect(() => {
    void screen.load();
  });
</script>

<TicketsScreen {screen} {workspaceId} />
