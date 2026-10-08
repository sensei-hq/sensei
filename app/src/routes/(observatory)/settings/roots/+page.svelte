<script lang="ts">
  import { onMount } from 'svelte';
  import { PageHeader } from '$lib/components';
  import RootsSection from '$lib/components/settings/RootsSection.svelte';
  import { senseiApi } from '$lib/api.js';
  import { appState } from '$lib/appstate.svelte.js';
  import { wizardState } from '$lib/wizard-state.svelte.js';

  // Outside setup nothing else loads the roots, so this screen reads them
  // itself (#247). A failed read is said; the cached list is kept.
  let failure = $state<string | null>(null);
  onMount(async () => {
    failure = await wizardState.reloadRoots(senseiApi(appState.port));
  });
</script>

<PageHeader kanji="庵" eyebrow="Settings" title="Roots" />
<div class="max-w-[820px] mx-auto px-12 pt-8 pb-16">
  {#if failure}
    <p class="text-sm text-warning mb-4" data-testid="roots-load-error">{failure}</p>
  {/if}
  <RootsSection />
</div>
