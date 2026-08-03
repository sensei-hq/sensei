<script lang="ts">
	import { goto, invalidateAll } from '$app/navigation';
	import { adoptContribution, createDojo, syncGithubOrgs, ClientApiError } from '$lib/client-data';
	import ScrPlaceholder from '$lib/components/screens/ScrPlaceholder.svelte';
	import ScrProjects from '$lib/components/screens/ScrProjects.svelte';
	import ScrConstitution from '$lib/components/screens/ScrConstitution.svelte';
	import ScrRulePacks from '$lib/components/screens/ScrRulePacks.svelte';
	import ScrMyDojos from '$lib/components/screens/ScrMyDojos.svelte';
	import ScrContributions from '$lib/components/screens/ScrContributions.svelte';
	import { youHref, orgHref } from '$lib/nav';
	import type { KitProject, KitDojo, KitDownstream } from '$lib/components/kit/types';

	// The personal-zone section screen — the non-inbox destinations (projects ·
	// rules · packs · dojos · contributions). Opening a project routes to the
	// preview drill-in; a dōjō row steps into its org console; Pin adopts a
	// downstream contribution; "Create or join" opens the create-a-dōjō form (F3a).
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

	// ── Create a dōjō (F3a) — the "Create or join" CTA opens this inline form; a
	// real create makes the caller admin and the new dōjō appears via a reload.
	let creating = $state(false);
	let cName = $state('');
	let cKind = $state('employer');
	let cErr = $state<string | null>(null);
	let cBusy = $state(false);
	const KINDS = ['employer', 'client', 'community', 'personal'];

	function openCreate() {
		creating = true;
		cName = '';
		cKind = 'employer';
		cErr = null;
	}
	async function submitCreate(e: Event) {
		e.preventDefault();
		if (!cName.trim() || cBusy) return;
		cBusy = true;
		cErr = null;
		try {
			await createDojo(cName.trim(), cKind);
			creating = false;
			await invalidateAll();
		} catch (err) {
			cErr = err instanceof ClientApiError ? err.message : 'could not create the dōjō';
		} finally {
			cBusy = false;
		}
	}

	// ── Auto-join GitHub-org dōjōs (F3c) — resolve the caller's GitHub orgs from
	// their OAuth session server-side and join any matching github/{org} dōjō.
	let ghBusy = $state(false);
	let ghMsg = $state<string | null>(null);
	async function syncGithub() {
		if (ghBusy) return;
		ghBusy = true;
		ghMsg = null;
		try {
			const r = await syncGithubOrgs();
			ghMsg = !r.synced
				? 'Sign in with GitHub to auto-join your org dōjōs.'
				: r.joined.length
					? `Joined ${r.joined.length} dōjō${r.joined.length > 1 ? 's' : ''} from GitHub.`
					: 'No new GitHub org dōjōs to join.';
			if (r.joined.length) await invalidateAll();
		} catch {
			ghMsg = 'Could not sync from GitHub — try again.';
		} finally {
			ghBusy = false;
		}
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
	<div class="flex items-center justify-end gap-3" style="padding: 12px 32px 0">
		{#if ghMsg}<span class="text-ink-mute text-xs">{ghMsg}</span>{/if}
		<button
			type="button"
			onclick={syncGithub}
			disabled={ghBusy}
			class="border-paper-edge text-ink-soft cursor-pointer rounded-lg border bg-transparent px-3 py-1.5 text-xs disabled:cursor-default disabled:opacity-50"
		>
			{ghBusy ? 'Syncing…' : 'Sync from GitHub'}
		</button>
	</div>
	<ScrMyDojos dojos={data.dojos} onOpen={openDojo} onCreateOrJoin={openCreate} />
	{#if creating}
		<div class="fixed inset-0 z-50 flex items-center justify-center p-4">
			<button
				type="button"
				class="bg-ink/40 absolute inset-0 cursor-default border-none"
				aria-label="Close"
				onclick={() => (creating = false)}
			></button>
			<form
				class="bg-paper border-paper-edge relative flex w-full max-w-md flex-col gap-4 rounded-2xl border p-6"
				onsubmit={submitCreate}
			>
				<div class="display text-lg">Create a dōjō</div>
				<label class="flex flex-col gap-1">
					<span class="text-ink-soft text-xs">Name</span>
					<input
						bind:value={cName}
						placeholder="Acme Corp"
						class="bg-paper border-paper-edge text-ink box-border w-full rounded-lg border px-3 py-2 text-sm"
					/>
				</label>
				<label class="flex flex-col gap-1">
					<span class="text-ink-soft text-xs">Kind</span>
					<select
						bind:value={cKind}
						class="bg-paper border-paper-edge text-ink box-border w-full rounded-lg border px-3 py-2 text-sm"
					>
						{#each KINDS as k (k)}
							<option value={k}>{k}</option>
						{/each}
					</select>
				</label>
				{#if cErr}
					<div class="text-danger text-xs">{cErr}</div>
				{/if}
				<div class="flex justify-end gap-2">
					<button
						type="button"
						onclick={() => (creating = false)}
						class="text-ink-mute cursor-pointer border-none bg-transparent px-3 py-2 text-sm">Cancel</button
					>
					<button
						type="submit"
						disabled={cBusy || !cName.trim()}
						class="bg-primary text-on-primary cursor-pointer rounded-lg border-none px-4 py-2 text-sm font-medium disabled:cursor-default disabled:opacity-50"
					>
						{cBusy ? 'Creating…' : 'Create'}
					</button>
				</div>
			</form>
		</div>
	{/if}
{:else if data.section === 'contributions'}
	<ScrContributions
		mine={data.contributionsMine}
		downstream={data.contributionsDownstream}
		onPin={pinContribution}
	/>
{:else}
	<ScrPlaceholder title={data.title} eyebrow="You" />
{/if}
