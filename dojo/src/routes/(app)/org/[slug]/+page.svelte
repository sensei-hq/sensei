<script lang="ts">
	import { goto } from '$app/navigation';
	import ScrOrgHome from '$lib/components/screens/ScrOrgHome.svelte';
	import { orgHref } from '$lib/nav';
	import type { KitWorkspace } from '$lib/components/kit/types';

	// The org-zone home (mockup ScrOrgHome) — the workspaces in this dōjō's
	// jurisdiction, an org "needs a maintainer" band, and the jurisdiction stat
	// row. Renders at /org/[slug]. The org is resolved from the URL slug against
	// the caller's real memberships (see +page.ts). Opening a workspace routes to the
	// org workspace preview at /org/[slug]/workspaces/[id] (URL-driven, like
	// /you/workspaces/[id]) — the shell keeps "Workspaces" active for that tail.
	let { data } = $props();

	function openWorkspace(p: KitWorkspace) {
		goto(orgHref(data.slug, 'workspaces') + '/' + p.id);
	}
</script>

<svelte:head><title>{data.orgName} · Dōjō</title></svelte:head>

<ScrOrgHome
	orgName={data.orgName}
	workspaces={data.workspaces}
	needs={data.needs}
	needsError={data.needsError}
	stats={data.stats}
	onOpenWorkspace={openWorkspace}
/>
