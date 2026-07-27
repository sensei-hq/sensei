<script lang="ts">
	import { goto } from '$app/navigation';
	import ScrYourWork from '$lib/components/dojo2/ScrYourWork.svelte';
	import { youHref } from '$lib/dojo2-nav';
	import type { KitProject, KitRun } from '$lib/components/kit/types';

	// The personal "Your work" landing (the signed-in dojo2 home). The band /
	// live-runs / active-projects render off the ported kit fixtures supplied by
	// +page.ts (presentational this chunk; real /v1 wiring lands later). Opening a
	// project routes to the constitution preview drill-in — the shell keeps
	// "Projects" active for the /you/projects/[id] tail; opening a live run routes
	// to the run detail at /you/runs/[run_id] (KitRun.id IS the run_id).
	let { data } = $props();

	function openProject(p: KitProject) {
		goto(youHref('projects') + '/' + p.id);
	}

	function openRun(r: KitRun) {
		goto(youHref('runs') + '/' + r.id);
	}
</script>

<svelte:head><title>Your work · Dōjō</title></svelte:head>

<ScrYourWork
	needsYou={data.needsYou}
	runs={data.runs}
	projects={data.projects}
	onOpenProject={openProject}
	onOpenRun={openRun}
/>
