<script lang="ts">
	import { goto, invalidateAll } from '$app/navigation';
	import {
		adoptContribution,
		provisionFromForge,
		setPackAdoption,
		setRepoElection
	} from '$lib/client-data';
	import ScrPlaceholder from '$lib/components/screens/ScrPlaceholder.svelte';
	import ScrWorkspaces from '$lib/components/screens/ScrWorkspaces.svelte';
	import ScrConstitution from '$lib/components/screens/ScrConstitution.svelte';
	import ScrRulePacks from '$lib/components/screens/ScrRulePacks.svelte';
	import ScrMyDojos from '$lib/components/screens/ScrMyDojos.svelte';
	import ScrSharing from '$lib/components/screens/ScrSharing.svelte';
	import ScrContributions from '$lib/components/screens/ScrContributions.svelte';
	import { youHref, orgHref } from '$lib/nav';
	import { YouSectionState, type YouApi } from '$lib/you-state.svelte';
	import type { KitWorkspace, KitDojo, KitDownstream, KitRulePack } from '$lib/components/kit/types';

	// The personal-zone section screen — the non-inbox destinations (workspaces ·
	// rules · packs · sharing · dojos · contributions). Presentation only: the
	// writes run in YouSectionState, which says why each one failed; the load is
	// +page.ts and every write re-reads it.
	let { data } = $props();

	const you = new YouSectionState();
	const api: YouApi = { adoptContribution, setPackAdoption, provisionFromForge, setRepoElection };

	// Opening a workspace routes to the preview drill-in; a dōjō row steps into its
	// org console (the slug is the org id, which /org/[slug] resolves).
	const openWorkspace = (p: KitWorkspace) => goto(youHref('workspaces') + '/' + p.id);
	const goPacks = () => goto(youHref('packs'));
	const openDojo = (d: KitDojo) => goto(orgHref(d.slug));

	const pinContribution = (item: KitDownstream) => you.pin(api, item, invalidateAll);
	const onPackToggle = (pack: KitRulePack, adopt: boolean) => you.togglePack(api, pack, adopt, invalidateAll);
	const syncGithub = () => you.syncGithub(api, invalidateAll);
	const elect = (repoKey: string, elected: boolean) => you.elect(api, repoKey, elected, invalidateAll);

	const sharingRepos = $derived(you.sharingRepos(data.repos ?? []));
	const sharingError = $derived(you.sharingError(data.reposError ?? null));
</script>

<svelte:head><title>{data.title} · Dōjō</title></svelte:head>

{#if data.section === 'workspaces'}
	<ScrWorkspaces
		workspaces={data.workspaces}
		error={data.workspacesError}
		showDojo={false}
		onOpenWorkspace={openWorkspace}
	/>
{:else if data.section === 'rules'}
	<ScrConstitution stance={data.stance} ladder={data.ladder} onGoPacks={goPacks} />
{:else if data.section === 'packs'}
	<ScrRulePacks packs={data.rulePacks} error={you.actionError} onToggle={onPackToggle} />
{:else if data.section === 'sharing'}
	<ScrSharing repos={sharingRepos} error={sharingError} pending={you.sharingPending} onToggle={elect} />
{:else if data.section === 'dojos'}
	<ScrMyDojos
		dojos={data.dojos}
		onOpen={openDojo}
		onSyncGithub={syncGithub}
		syncing={you.github.busy}
		syncMessage={you.github.message}
	/>
{:else if data.section === 'contributions'}
	<ScrContributions
		mine={data.contributionsMine}
		downstream={data.contributionsDownstream}
		error={you.actionError}
		onPin={pinContribution}
	/>
{:else}
	<ScrPlaceholder title={data.title} eyebrow="You" />
{/if}
