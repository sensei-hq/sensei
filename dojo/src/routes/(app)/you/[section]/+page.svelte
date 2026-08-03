<script lang="ts">
	import { goto, invalidateAll } from '$app/navigation';
	import { adoptContribution } from '$lib/client-data';
	import ScrPlaceholder from '$lib/components/screens/ScrPlaceholder.svelte';
	import ScrProjects from '$lib/components/screens/ScrProjects.svelte';
	import ScrConstitution from '$lib/components/screens/ScrConstitution.svelte';
	import ScrRulePacks from '$lib/components/screens/ScrRulePacks.svelte';
	import ScrMyDojos from '$lib/components/screens/ScrMyDojos.svelte';
	import ScrContributions from '$lib/components/screens/ScrContributions.svelte';
	import { youHref, orgHref } from '$lib/nav';
	import type { KitProject, KitDojo, KitDownstream } from '$lib/components/kit/types';

	// The personal-zone section screen — the non-inbox destinations (projects ·
	// rules · packs · dojos · contributions). The relay surfaces (approve · decide
	// · chat) folded into the Inbox; their old URLs redirect there. Opening a
	// project routes to the preview drill-in; Constitution's "Rule packs →" routes
	// to /you/packs.
	let { data } = $props();

	function openProject(p: KitProject) {
		goto(youHref('projects') + '/' + p.id);
	}
	function goPacks() {
		goto(youHref('packs'));
	}
	// A dōjō row steps into that dōjō's org console (my-dojos resolved design Q3);
	// the slug is the org id, which /org/[slug] resolves via orgBySlug.
	function openDojo(d: KitDojo) {
		goto(orgHref(d.slug));
	}
	// Pin an approved-for-you contribution → the real adopt write, then re-load so
	// the row flips to "adopted" from the source of truth (no fabricated state).
	async function pinContribution(item: KitDownstream) {
		if (!item.id) return;
		await adoptContribution(item.id);
		await invalidateAll();
	}
</script>

<svelte:head><title>{data.title} · Dōjō</title></svelte:head>

{#if data.section === 'projects'}
	<ScrProjects
		projects={data.projects}
		error={data.projectsError}
		showDojo={false}
		onOpenProject={openProject}
	/>
{:else if data.section === 'rules'}
	<ScrConstitution stance={data.stance} ladder={data.ladder} onGoPacks={goPacks} />
{:else if data.section === 'packs'}
	<ScrRulePacks packs={data.rulePacks} />
{:else if data.section === 'dojos'}
	<ScrMyDojos dojos={data.dojos} onOpen={openDojo} />
{:else if data.section === 'contributions'}
	<ScrContributions
		mine={data.contributionsMine}
		downstream={data.contributionsDownstream}
		onPin={pinContribution}
	/>
{:else}
	<ScrPlaceholder title={data.title} eyebrow="You" />
{/if}
