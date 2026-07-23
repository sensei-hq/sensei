<script lang="ts">
	import DojoRulePreview from './DojoRulePreview.svelte';
	import { createPreviewStore } from '$lib/preview-state.svelte';

	// The /console/preview route. Reachable for every signed-in user — previewing
	// the ladder resolved for a project works solo (no Dōjō membership gate), so
	// unlike the org-scoped screens there is no join-empty branch (DJ1). Composes
	// the presentational preview with a fresh selection store (default project =
	// the Globex client engagement, the richest resolution case).

	// One store per page load so the selected project / classification override
	// don't leak across navigations.
	const store = createPreviewStore();
</script>

<svelte:head>
	<title>Effective constitution · Dōjō</title>
</svelte:head>

<DojoRulePreview {store} />
