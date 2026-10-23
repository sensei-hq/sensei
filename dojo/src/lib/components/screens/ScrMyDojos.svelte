<script lang="ts">
	import { Screen } from '@sensei/ui';
	import { Button } from '@rokkit/ui';
	import { SectionHead, ListSection, MyDojoRow, EmptyState, AzureConnect } from '$lib/components/kit';
	import type { AzureConnectState } from '$lib/azure-connect-state.svelte';
	import type { KitDojo } from '$lib/components/kit/types';
	import { groupDojos } from '$lib/personal-view';

	// My dōjōs (mockup ScrMyDojos) — the memberships you belong to, grouped
	// employer → clients → communities (empty groups dropped). A dōjō is how a
	// team shares what it learns; your role in each is derived from git and only
	// ever adds capability — working solo needs none of them. Degrades to an
	// empty state when you have no memberships (DJ1). A dōjō is an organization's
	// or a person's and arrives through GitHub or an invite — none is made by hand
	// (#307 D11). Presentational: the page supplies the memberships; `onOpen`
	// bubbles a row open.
	let {
		dojos = [],
		onOpen,
		onSyncGithub,
		syncing = false,
		syncMessage = null,
		azure
	}: {
		dojos?: KitDojo[];
		onOpen?: (dojo: KitDojo) => void;
		/** Re-read the memberships from GitHub; offered only when given. */
		onSyncGithub?: () => void;
		/** A sync is in flight. */
		syncing?: boolean;
		/** What the last sync came to, said beside the button. */
		syncMessage?: string | null;
		/** Import an Azure DevOps organization as its own dōjō (#308 R5b) — how an
		 *  Azure organization's dōjō is first made. Offered only when given. */
		azure?: AzureConnectState;
	} = $props();

	const groups = $derived(groupDojos(dojos));

	// The count only reads if there IS one, so the sentence is built rather than
	// branched inline — `description` is a plain string prop.
	const description = $derived(
		dojos.length
			? `A dōjō is how a team shares what it learns. You belong to ${dojos.length} — your role in each is derived from git and only ever adds capability. Working solo needs none of them.`
			: 'A dōjō is how a team shares what it learns. Your role in each is derived from git and only ever adds capability. Working solo needs none of them.'
	);
</script>

<Screen>
	{#snippet header()}
		<div class="px-4 pt-4 md:px-8 md:pt-8">
			<SectionHead
				kanji="結"
				eyebrow="You · membership"
				title="My dōjōs"
				count={dojos.length}
				{description}
			>
				{#snippet right()}
					<div class="flex items-center gap-3">
						{#if syncMessage}<span class="text-ink-mute text-xs">{syncMessage}</span>{/if}
						{#if onSyncGithub}
							<Button style="outline" size="sm" onclick={onSyncGithub} disabled={syncing}>
								{syncing ? 'Syncing…' : 'Sync from GitHub'}
							</Button>
						{/if}
					</div>
				{/snippet}
			</SectionHead>
		</div>
	{/snippet}
	<div class="flex flex-col p-4 gap-4 md:p-8 md:gap-6">
		{#if groups.length}
			{#each groups as group (group.kind)}
				<ListSection icon={group.icon} title={group.label} count={group.items.length}>
					{#each group.items as dojo (dojo.slug)}
						<MyDojoRow {dojo} onopen={onOpen} />
					{/each}
				</ListSection>
			{/each}
		{:else}
			<EmptyState kanji="空" title="No dōjōs yet.">
				A dōjō comes from your GitHub or Azure DevOps organizations, or an invite — each organization you belong
				to has one, and you have a personal one. Sync from GitHub, or connect Azure DevOps and import an
				organization you administer, to set them up. Working solo needs none of them.
			</EmptyState>
		{/if}
		{#if azure}
			<AzureConnect
				state={azure}
				intro="Import an Azure DevOps organization you administer as its own dōjō: each of its projects becomes a workspace in it."
			/>
		{/if}
	</div>
</Screen>
