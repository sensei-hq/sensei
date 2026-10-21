<script lang="ts">
	import { goto, invalidateAll } from '$app/navigation';
	import {
		adoptContribution,
		createDojo,
		provisionFromForge,
		setPackAdoption,
		setRepoElection
	} from '$lib/client-data';
	import ScrPlaceholder from '$lib/components/screens/ScrPlaceholder.svelte';
	import ScrProjects from '$lib/components/screens/ScrProjects.svelte';
	import ScrConstitution from '$lib/components/screens/ScrConstitution.svelte';
	import ScrRulePacks from '$lib/components/screens/ScrRulePacks.svelte';
	import ScrMyDojos from '$lib/components/screens/ScrMyDojos.svelte';
	import ScrSharing from '$lib/components/screens/ScrSharing.svelte';
	import ScrContributions from '$lib/components/screens/ScrContributions.svelte';
	import { youHref, orgHref } from '$lib/nav';
	import { DOJO_KINDS, YouSectionState, type YouApi } from '$lib/you-state.svelte';
	import type { KitProject, KitDojo, KitDownstream, KitRulePack } from '$lib/components/kit/types';

	// The personal-zone section screen — the non-inbox destinations (projects ·
	// rules · packs · sharing · dojos · contributions). Presentation only: the
	// writes run in YouSectionState, which says why each one failed; the load is
	// +page.ts and every write re-reads it.
	let { data } = $props();

	const you = new YouSectionState();
	const api: YouApi = { adoptContribution, setPackAdoption, createDojo, provisionFromForge, setRepoElection };

	// Opening a project routes to the preview drill-in; a dōjō row steps into its
	// org console (the slug is the org id, which /org/[slug] resolves).
	const openProject = (p: KitProject) => goto(youHref('projects') + '/' + p.id);
	const goPacks = () => goto(youHref('packs'));
	const openDojo = (d: KitDojo) => goto(orgHref(d.slug));

	const pinContribution = (item: KitDownstream) => you.pin(api, item, invalidateAll);
	const onPackToggle = (pack: KitRulePack, adopt: boolean) => you.togglePack(api, pack, adopt, invalidateAll);
	const syncGithub = () => you.syncGithub(api, invalidateAll);
	const elect = (repoKey: string, elected: boolean) => you.elect(api, repoKey, elected, invalidateAll);
	function submitCreate(e: Event) {
		e.preventDefault();
		void you.submitCreate(api, invalidateAll);
	}

	const sharingRepos = $derived(you.sharingRepos(data.repos ?? []));
	const sharingError = $derived(you.sharingError(data.reposError ?? null));
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
	<ScrRulePacks packs={data.rulePacks} error={you.actionError} onToggle={onPackToggle} />
{:else if data.section === 'sharing'}
	<ScrSharing repos={sharingRepos} error={sharingError} pending={you.sharingPending} onToggle={elect} />
{:else if data.section === 'dojos'}
	<ScrMyDojos
		dojos={data.dojos}
		onOpen={openDojo}
		onCreateOrJoin={() => you.openCreate()}
		onSyncGithub={syncGithub}
		syncing={you.github.busy}
		syncMessage={you.github.message}
	/>
	{#if you.create.open}
		<div class="fixed inset-0 z-50 flex items-center justify-center p-4">
			<button
				type="button"
				class="bg-ink/40 absolute inset-0 cursor-default border-none"
				aria-label="Close"
				onclick={() => you.closeCreate()}
			></button>
			<form
				class="bg-paper border-paper-edge relative flex w-full max-w-md flex-col gap-4 rounded-2xl border p-6"
				onsubmit={submitCreate}
			>
				<div class="display text-lg">Create a team dōjō</div>
				<p class="text-ink-soft text-xs">
					A team you name yourself. A GitHub organisation's dōjō is made when one of its admins signs in.
				</p>
				<label class="flex flex-col gap-1">
					<span class="text-ink-soft text-xs">Name</span>
					<input
						bind:value={you.create.name}
						placeholder="Platform guild"
						class="bg-paper border-paper-edge text-ink box-border w-full rounded-lg border px-3 py-2 text-sm"
					/>
				</label>
				<label class="flex flex-col gap-1">
					<span class="text-ink-soft text-xs">Kind</span>
					<select
						bind:value={you.create.kind}
						class="bg-paper border-paper-edge text-ink box-border w-full rounded-lg border px-3 py-2 text-sm"
					>
						{#each DOJO_KINDS as k (k)}
							<option value={k}>{k}</option>
						{/each}
					</select>
				</label>
				{#if you.create.error}
					<div class="text-danger text-xs">{you.create.error}</div>
				{/if}
				<div class="flex justify-end gap-2">
					<button
						type="button"
						onclick={() => you.closeCreate()}
						class="text-ink-mute cursor-pointer border-none bg-transparent px-3 py-2 text-sm">Cancel</button
					>
					<button
						type="submit"
						disabled={you.create.busy || !you.create.name.trim()}
						class="bg-primary text-on-primary cursor-pointer rounded-lg border-none px-4 py-2 text-sm font-medium disabled:cursor-default disabled:opacity-50"
					>
						{you.create.busy ? 'Creating…' : 'Create'}
					</button>
				</div>
			</form>
		</div>
	{/if}
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
