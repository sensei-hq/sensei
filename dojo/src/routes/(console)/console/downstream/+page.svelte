<script lang="ts">
	import DojoDeveloperDownstream from '$lib/components/DojoDeveloperDownstream.svelte';

	// The /console/downstream route — "For me" in the personal seat. A personal
	// surface: reachable for every signed-in user with NO join-gate (DJ1).
	// Approved teachings only flow to members, so the section reads
	// `hasMembership` from the console layout load and shows the honest empty
	// state ("no teachings yet") for a solo contributor. Presentational — the
	// sample teachings come from `$lib/developer-data`; `/v1` wiring is deferred.
	let { data } = $props();
</script>

<svelte:head>
	<title>For me · Dōjō</title>
</svelte:head>

<DojoDeveloperDownstream hasMembership={data.hasMembership} />
