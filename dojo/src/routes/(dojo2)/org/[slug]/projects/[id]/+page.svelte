<script lang="ts">
	import { goto } from '$app/navigation';
	import ScrProjectPreview from '$lib/components/dojo2/ScrProjectPreview.svelte';
	import { orgHref } from '$lib/dojo2-nav';

	// The org project-constitution preview drill-in (mockup ScrProjectPreview) —
	// the resolved ladder + discarded conflicts for one project in the dōjō's
	// jurisdiction. Reached from the org home or the org project list
	// (onOpenProject → /org/[slug]/projects/[id]); the back header returns to the
	// org project list. The shell keeps "Projects" active for this tail, so the
	// drill reads as an in-place focus rather than a context switch. Reuses the same
	// ScrProjectPreview pattern as /you/projects/[id].
	let { data } = $props();

	function back() {
		goto(orgHref(data.slug, 'projects'));
	}
</script>

<svelte:head><title>{data.project.name} · {data.orgName} · Dōjō</title></svelte:head>

<ScrProjectPreview
	project={data.project}
	ladder={data.ladder}
	conflicts={data.conflicts}
	onBack={back}
/>
