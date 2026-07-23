<script lang="ts">
	import { goto } from '$app/navigation';
	import ScrOrgHome from '$lib/components/dojo2/ScrOrgHome.svelte';
	import { orgHref } from '$lib/dojo2-nav';
	import type { KitProject } from '$lib/components/kit/types';

	// The org-zone home (mockup ScrOrgHome) — the projects in this dōjō's
	// jurisdiction, an org "needs a maintainer" band, and the jurisdiction stat
	// row. Renders at /org/[slug]. The org is resolved from the URL slug against
	// the caller's real memberships (see +page.ts). Opening a project routes to the
	// org project preview at /org/[slug]/projects/[id] (URL-driven, like
	// /you/projects/[id]) — the shell keeps "Projects" active for that tail.
	let { data } = $props();

	function openProject(p: KitProject) {
		goto(orgHref(data.slug, 'projects') + '/' + p.id);
	}
</script>

<svelte:head><title>{data.orgName} · Dōjō</title></svelte:head>

<ScrOrgHome
	orgName={data.orgName}
	projects={data.projects}
	needs={data.needs}
	stats={data.stats}
	onOpenProject={openProject}
/>
