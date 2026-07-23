<script lang="ts">
	import ScrPlaceholder from '$lib/components/dojo2/ScrPlaceholder.svelte';

	// Personal-zone placeholder ("coming in the rebuild") for a NAV_YOU section.
	let { data } = $props();
</script>

<svelte:head><title>{data.title} · Dōjō</title></svelte:head>

<ScrPlaceholder title={data.title} eyebrow="You" />
