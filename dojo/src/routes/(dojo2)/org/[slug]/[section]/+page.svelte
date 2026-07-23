<script lang="ts">
	import ScrPlaceholder from '$lib/components/dojo2/ScrPlaceholder.svelte';

	// Org-zone placeholder ("coming in the rebuild") for a NAV_ORG section.
	let { data } = $props();
</script>

<svelte:head><title>{data.title} · {data.orgName} · Dōjō</title></svelte:head>

<ScrPlaceholder title={data.title} eyebrow={data.orgName} />
