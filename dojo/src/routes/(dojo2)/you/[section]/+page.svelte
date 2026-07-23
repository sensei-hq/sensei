<script lang="ts">
	import { goto } from '$app/navigation';
	import ScrPlaceholder from '$lib/components/dojo2/ScrPlaceholder.svelte';
	import ScrProjects from '$lib/components/dojo2/ScrProjects.svelte';
	import ScrRelayWatch from '$lib/components/dojo2/ScrRelayWatch.svelte';
	import ScrRelayApprove from '$lib/components/dojo2/ScrRelayApprove.svelte';
	import ScrRelayDecide from '$lib/components/dojo2/ScrRelayDecide.svelte';
	import ScrRelayChat from '$lib/components/dojo2/ScrRelayChat.svelte';
	import ScrConstitution from '$lib/components/dojo2/ScrConstitution.svelte';
	import ScrRulePacks from '$lib/components/dojo2/ScrRulePacks.svelte';
	import ScrMyDojos from '$lib/components/dojo2/ScrMyDojos.svelte';
	import ScrContributions from '$lib/components/dojo2/ScrContributions.svelte';
	import { youHref } from '$lib/dojo2-nav';
	import type { KitProject } from '$lib/components/kit/types';

	// The personal-zone section screen. Dispatches to the real screens for the
	// ported sections (projects · runs · approve · decide · chat · rules · packs ·
	// dojos · contributions) off the kit fixtures the loader supplies — NAV_YOU is
	// now complete. Opening a project routes to the preview drill-in at
	// /you/projects/[id] (an in-shell route so the URL stays the source of truth —
	// the shell keeps "Projects" active for the tail). Constitution's "Rule packs
	// →" link routes to /you/packs.
	let { data } = $props();

	// The screen actions are presentational this chunk — a later chunk wires them
	// to live `/v1` mutations. The two real navigations are opening a project and
	// following the Constitution → Rule packs link.
	function openProject(p: KitProject) {
		goto(youHref('projects') + '/' + p.id);
	}

	function goPacks() {
		goto(youHref('packs'));
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
{:else if data.section === 'rules'}
	<ScrConstitution stance={data.stance} ladder={data.ladder} onGoPacks={goPacks} />
{:else if data.section === 'packs'}
	<ScrRulePacks packs={data.rulePacks} />
{:else if data.section === 'dojos'}
	<ScrMyDojos dojos={data.dojos} />
{:else if data.section === 'contributions'}
	<ScrContributions
		mine={data.contributionsMine}
		downstream={data.contributionsDownstream}
		stat={data.contributionsStat}
	/>
{:else}
	<ScrPlaceholder title={data.title} eyebrow="You" />
{/if}
