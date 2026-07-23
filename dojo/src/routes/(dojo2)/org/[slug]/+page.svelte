<script lang="ts">
	import ScrPlaceholder from '$lib/components/dojo2/ScrPlaceholder.svelte';

	// The org-zone home (mockup ScrOrgHome) — a placeholder this chunk. The org's
	// name is resolved from the URL slug against the caller's memberships (see
	// +page.ts); the projects / needs-a-maintainer band port in a later chunk.
	let { data } = $props();
	const name = $derived(data.orgName);
</script>

<svelte:head><title>{name} · Dōjō</title></svelte:head>

<ScrPlaceholder title="Home" eyebrow={name} />
