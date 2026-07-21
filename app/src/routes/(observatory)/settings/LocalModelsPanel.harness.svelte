<script lang="ts">
  import LocalModelsPanel from './LocalModelsPanel.svelte';
  import type { LocalModels } from './local-models.svelte.js';

  // The panel is driven by an injected, fixture-backed controller so the spec
  // renders any phase deterministically (a real pull is blocked upstream).
  let { controller }: { controller: LocalModels } = $props();
</script>

<LocalModelsPanel {controller} />
