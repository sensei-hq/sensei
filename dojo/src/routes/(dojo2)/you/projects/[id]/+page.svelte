<script lang="ts">
	import { goto } from '$app/navigation';
	import ScrProjectPreview from '$lib/components/dojo2/ScrProjectPreview.svelte';
	import { youHref } from '$lib/dojo2-nav';

	// The project-constitution preview drill-in (mockup ScrProjectPreview) — the
	// resolved ladder + discarded conflicts for one project. Reached from the
	// projects list or the landing (onOpenProject → /you/projects/[id]); the back
	// header returns to the list. The shell keeps "Projects" active for this tail,
	// so the drill reads as an in-place focus rather than a context switch.
	let { data } = $props();

	function back() {
		goto(youHref('projects'));
	}
</script>

<svelte:head><title>{data.project.name} · Dōjō</title></svelte:head>

<ScrProjectPreview
	project={data.project}
	ladder={data.ladder}
	conflicts={data.conflicts}
	onBack={back}
/>
