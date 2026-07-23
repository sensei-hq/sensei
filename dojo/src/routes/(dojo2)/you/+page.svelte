<script lang="ts">
	import ScrYourWork from '$lib/components/dojo2/ScrYourWork.svelte';

	// The personal "Your work" landing (the signed-in dojo2 home). The band /
	// live-runs / active-projects render off the ported kit fixtures supplied by
	// +page.ts (presentational this chunk; real /v1 wiring lands later). Opening a
	// project is a no-op until the project-preview screen ports — the row stays
	// inert rather than routing to a 404.
	let { data } = $props();
</script>

<svelte:head><title>Your work · Dōjō</title></svelte:head>

<ScrYourWork needsYou={data.needsYou} runs={data.runs} projects={data.projects} />
