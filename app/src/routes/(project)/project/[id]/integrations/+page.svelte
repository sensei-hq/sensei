<script lang="ts">
  // 繋 Integrations — the project's issue tracker. The state is client-side
  // because the screen polls while a first sync runs; it never reads a failed
  // fetch as "not connected" (see integrations-state.svelte.ts).
  import { page } from '$app/state';
  import { senseiApi } from '$lib/api.js';
  import { appState } from '$lib/appstate.svelte.js';
  import IntegrationsScreen from './IntegrationsScreen.svelte';
  import { TrackerIntegration } from './integrations-state.svelte.js';

  // One screen state per project: switching project on this route gets a fresh
  // one, and the old one stops polling.
  const screen = $derived(new TrackerIntegration(senseiApi(appState.port), page.params.id ?? ''));
  $effect(() => {
    const s = screen;
    void s.load();
    return () => s.dispose();
  });
</script>

<IntegrationsScreen {screen} />
