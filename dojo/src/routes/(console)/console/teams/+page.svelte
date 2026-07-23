<script lang="ts">
	import DojoDeveloperTeams from '$lib/components/DojoDeveloperTeams.svelte';

	// The /console/teams route — "My teams" in the personal (developer) seat. A
	// personal surface: reachable for every signed-in user with NO join-gate
	// (DJ1). Driven by the caller's real memberships from the console layout load
	// (data.memberships); when there are none the section shows the honest empty
	// state itself. Presentational — no page-level `/v1` fetch.
	let { data } = $props();
</script>

<svelte:head>
	<title>My teams · Dōjō</title>
</svelte:head>

<DojoDeveloperTeams memberships={data.memberships} />
