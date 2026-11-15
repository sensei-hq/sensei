<script lang="ts">
  // 納 Delivery Metrics — each sprint board's sprints and the AI work in your sessions
  // behind them. Client-side state: selecting a sprint reads its tickets on demand.
  import { page } from '$app/state';
  import { senseiApi } from '$lib/api.js';
  import { appState } from '$lib/appstate.svelte.js';
  import DeliveryScreen from './DeliveryScreen.svelte';
  import { DeliveryState } from './delivery-state.svelte.js';

  const workspaceId = $derived(page.params.id ?? '');
  // One screen state per workspace: switching workspace on this route starts fresh.
  const screen = $derived(new DeliveryState(senseiApi(appState.port), workspaceId));
  $effect(() => {
    void screen.load();
  });
</script>

<DeliveryScreen {screen} {workspaceId} />
