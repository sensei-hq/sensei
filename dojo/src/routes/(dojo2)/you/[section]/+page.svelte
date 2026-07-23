<script lang="ts">
	import { goto } from '$app/navigation';
	import ScrPlaceholder from '$lib/components/dojo2/ScrPlaceholder.svelte';
	import ScrProjects from '$lib/components/dojo2/ScrProjects.svelte';
	import ScrRelayWatch from '$lib/components/dojo2/ScrRelayWatch.svelte';
	import ScrRelayApprove from '$lib/components/dojo2/ScrRelayApprove.svelte';
	import ScrRelayDecide from '$lib/components/dojo2/ScrRelayDecide.svelte';
	import ScrRelayChat from '$lib/components/dojo2/ScrRelayChat.svelte';
	import { youHref } from '$lib/dojo2-nav';
	import type { KitProject } from '$lib/components/kit/types';

	// The personal-zone section screen. Dispatches to the real chunk-2 screens for
	// the ported sections (projects · runs · approve · decide · chat) off the kit
	// fixtures the loader supplies; every other NAV_YOU destination still renders
	// the "coming in the rebuild" placeholder. Opening a project routes to the
	// preview drill-in at /you/projects/[id] (an in-shell route so the URL stays
	// the source of truth — the shell keeps "Projects" active for the tail).
	let { data } = $props();

	// The relay actions are presentational this chunk — a later chunk wires them to
	// live `/v1` mutations. Routing to the preview is the one real navigation.
	function openProject(p: KitProject) {
		goto(youHref('projects') + '/' + p.id);
	}
</script>

<svelte:head><title>{data.title} · Dōjō</title></svelte:head>

{#if data.section === 'projects'}
	<ScrProjects projects={data.projects} showDojo={false} onOpenProject={openProject} />
{:else if data.section === 'runs'}
	<ScrRelayWatch runs={data.runs} />
{:else if data.section === 'approve'}
	<ScrRelayApprove gates={data.gates} />
{:else if data.section === 'decide'}
	<ScrRelayDecide decisions={data.decisions} />
{:else if data.section === 'chat'}
	<ScrRelayChat thread={data.chat} me={data.me} />
{:else}
	<ScrPlaceholder title={data.title} eyebrow="You" />
{/if}
