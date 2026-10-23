<script lang="ts">
	import { goto } from '$app/navigation';
	import ScrWorkspacePreview from '$lib/components/screens/ScrWorkspacePreview.svelte';
	import { youHref } from '$lib/nav';

	// The workspace-constitution preview drill-in (mockup ScrWorkspacePreview) — the
	// resolved ladder + discarded conflicts for one workspace. Reached from the
	// workspaces list or the landing (onOpenWorkspace → /you/workspaces/[id]); the back
	// header returns to the list. The shell keeps "Workspaces" active for this tail,
	// so the drill reads as an in-place focus rather than a context switch.
	let { data } = $props();

	function back() {
		goto(youHref('workspaces'));
	}
</script>

<svelte:head><title>{data.workspace.name} · Dōjō</title></svelte:head>

<ScrWorkspacePreview
	workspace={data.workspace}
	ladder={data.ladder}
	conflicts={data.conflicts}
	error={data.constitutionError}
	onBack={back}
/>
