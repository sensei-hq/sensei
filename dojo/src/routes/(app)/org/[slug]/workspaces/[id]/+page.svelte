<script lang="ts">
	import { goto } from '$app/navigation';
	import ScrWorkspacePreview from '$lib/components/screens/ScrWorkspacePreview.svelte';
	import { orgHref } from '$lib/nav';

	// The org workspace-constitution preview drill-in (mockup ScrWorkspacePreview) —
	// the resolved ladder + discarded conflicts for one workspace in the dōjō's
	// jurisdiction. Reached from the org home or the org workspace list
	// (onOpenWorkspace → /org/[slug]/workspaces/[id]); the back header returns to the
	// org workspace list. The shell keeps "Workspaces" active for this tail, so the
	// drill reads as an in-place focus rather than a context switch. Reuses the same
	// ScrWorkspacePreview pattern as /you/workspaces/[id].
	let { data } = $props();

	function back() {
		goto(orgHref(data.slug, 'workspaces'));
	}
</script>

<svelte:head><title>{data.workspace.name} · {data.orgName} · Dōjō</title></svelte:head>

<ScrWorkspacePreview
	workspace={data.workspace}
	ladder={data.ladder}
	conflicts={data.conflicts}
	onBack={back}
/>
