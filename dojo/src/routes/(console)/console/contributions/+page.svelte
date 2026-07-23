<script lang="ts">
	import DojoDeveloperContributions from '$lib/components/DojoDeveloperContributions.svelte';

	// The /console/contributions route — "My contributions" in the personal seat.
	// A personal surface: reachable for every signed-in user with NO join-gate
	// (DJ1). Upstream sends only exist once you belong to a Dōjō, so the section
	// reads `hasMembership` from the console layout load and shows the honest
	// empty state ("nothing shared upstream yet") for a solo contributor.
	// Presentational — the sample sends come from `$lib/developer-data`; `/v1`
	// wiring to real `dojo.shared_rules` is deferred.
	let { data } = $props();
</script>

<svelte:head>
	<title>My contributions · Dōjō</title>
</svelte:head>

<DojoDeveloperContributions hasMembership={data.hasMembership} />
