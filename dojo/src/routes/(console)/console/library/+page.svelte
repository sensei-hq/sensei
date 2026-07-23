<script lang="ts">
	import DojoLibrary from './DojoLibrary.svelte';
	import { createLibraryStore } from '$lib/library-state.svelte';

	// The /console/library route. Reachable for every signed-in user — seeding a
	// constitution from proven rules works solo (no Dōjō membership gate), so
	// unlike the org-scoped screens there is no join-empty branch. Composes the
	// presentational browser with a fresh working-selection store and the scope
	// name resolved by the load (the org, or a neutral label when solo).
	let { data } = $props();

	// One store per page load so selection state doesn't leak across navigations.
	const store = createLibraryStore();
</script>

<svelte:head>
	<title>Constitution library · Dōjō</title>
</svelte:head>

<DojoLibrary {store} scopeName={data.scopeName} />
