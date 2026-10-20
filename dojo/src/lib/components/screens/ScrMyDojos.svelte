<script lang="ts">
	import { Screen } from '@sensei/ui';
	import { Button } from '@rokkit/ui';
	import { SectionHead, Btn, ListSection, MyDojoRow, EmptyState } from '$lib/components/kit';
	import type { KitDojo } from '$lib/components/kit/types';
	import { groupDojos } from '$lib/personal-view';

	// My dōjōs (mockup ScrMyDojos) — the memberships you belong to, grouped
	// employer → clients → communities (empty groups dropped). A dōjō is how a
	// team shares what it learns; your role in each is derived from git and only
	// ever adds capability — working solo needs none of them. Degrades to an
	// honest empty state when you have no memberships (DJ1). Presentational: the
	// page supplies the memberships (kit fixtures this chunk); `onOpen` bubbles a
	// row open and `onCreateOrJoin` fires the create/join affordance.
	let {
		dojos = [],
		onOpen,
		onCreateOrJoin,
		onSyncGithub,
		syncing = false,
		syncMessage = null
	}: {
		dojos?: KitDojo[];
		onOpen?: (dojo: KitDojo) => void;
		onCreateOrJoin?: () => void;
		/** Re-read the memberships from GitHub; offered only when given. */
		onSyncGithub?: () => void;
		/** A sync is in flight. */
		syncing?: boolean;
		/** What the last sync came to, said beside the button. */
		syncMessage?: string | null;
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
						<Btn size="sm" icon="add-circle" onclick={onCreateOrJoin}>Create or join</Btn>
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
			<EmptyState kanji="空" title="No memberships yet — create or join a Dōjō.">
				You can work entirely solo — your rules and projects stay on your machine. Join a dōjō when a
				team wants to share what it learns.
				{#snippet action()}
					<Btn size="sm" icon="add-circle" onclick={onCreateOrJoin}>Create or join a dōjō</Btn>
				{/snippet}
			</EmptyState>
		{/if}
	</div>
</Screen>
