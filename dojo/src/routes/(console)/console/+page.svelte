<script lang="ts">
	import DojoOverview from './DojoOverview.svelte';
	import DojoPersonalHome from '$lib/components/DojoPersonalHome.svelte';

	// The console index (DJ1). Branches on the caller's real membership
	// (`hasMembership`, from the layout server load — the membership fact, NOT a
	// tenant fallback):
	//   · membership-less → the solo personal home (no tenant, no `/v1/t/…` call)
	//   · a member        → today's maintainer Overview (extracted intact into
	//                        DojoOverview so the member path is unchanged)
	let { data } = $props();
</script>

<svelte:head>
	<title>{data.hasMembership ? 'Overview · Dōjō console' : 'Your work · Dōjō'}</title>
</svelte:head>

{#if data.hasMembership}
	<DojoOverview queue={data.queue} org={data.org} error={data.error} />
{:else}
	<DojoPersonalHome user={data.user} />
{/if}
