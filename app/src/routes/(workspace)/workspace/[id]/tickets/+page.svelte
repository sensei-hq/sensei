<script lang="ts">
  // 札 Tickets — the workspace's tickets and their AI usage, filtered by sprint.
  // Client-side state: selecting a sprint or a ticket reads on demand.
  // `?board=<tracker>&ticket=<key>` opens that ticket (a link from Delivery Metrics).
  import { page } from '$app/state';
  import { untrack } from 'svelte';
  import { senseiApi } from '$lib/api.js';
  import { appState } from '$lib/appstate.svelte.js';
  import TicketsScreen from './TicketsScreen.svelte';
  import { TicketsState } from './tickets-state.svelte.js';

  const workspaceId = $derived(page.params.id ?? '');
  // One screen state per workspace: switching workspace on this route starts fresh.
  const screen = $derived(new TicketsState(senseiApi(appState.port), workspaceId));
  $effect(() => {
    const s = screen;
    const ticket = untrack(() => page.url.searchParams.get('ticket'));
    if (ticket) s.followLink(untrack(() => page.url.searchParams.get('board')), ticket);
    void s.load();
  });
</script>

<TicketsScreen {screen} {workspaceId} />
