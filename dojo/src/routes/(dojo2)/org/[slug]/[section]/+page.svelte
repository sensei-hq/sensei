<script lang="ts">
	import { goto } from '$app/navigation';
	import ScrPlaceholder from '$lib/components/dojo2/ScrPlaceholder.svelte';
	import ScrOrgLadder from '$lib/components/dojo2/ScrOrgLadder.svelte';
	import ScrProjects from '$lib/components/dojo2/ScrProjects.svelte';
	import { orgHref } from '$lib/dojo2-nav';
	import type { KitProject } from '$lib/components/kit/types';

	// The org-zone section screen. Dispatches to the real Overview screens for the
	// ported sections (ladder · projects) off the kit fixtures the loader supplies;
	// every other NAV_ORG destination still renders the "coming in the rebuild"
	// placeholder (the role consoles land in the next group). Opening a project
	// routes to the org project preview at /org/[slug]/projects/[id] (an in-shell
	// route so the URL stays the source of truth — the shell keeps "Projects"
	// active for the tail).
	let { data } = $props();

	function openProject(p: KitProject) {
		goto(orgHref(data.slug, 'projects') + '/' + p.id);
	}
</script>

<svelte:head><title>{data.title} · {data.orgName} · Dōjō</title></svelte:head>

{#if data.section === 'ladder'}
	<ScrOrgLadder orgName={data.orgName} sections={data.sections} />
{:else if data.section === 'projects'}
	<ScrProjects
		projects={data.projects}
		showDojo={false}
		eyebrow={data.orgName + ' · jurisdiction'}
		title="Projects"
		onOpenProject={openProject}
	/>
{:else}
	<ScrPlaceholder title={data.title} eyebrow={data.orgName} />
{/if}
