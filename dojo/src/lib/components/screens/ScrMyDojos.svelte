<script lang="ts">
	import { SectionHead, Banner, Btn, ListSection, MyDojoRow, EmptyState } from '$lib/components/kit';
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
		onCreateOrJoin
	}: {
		dojos?: KitDojo[];
		onOpen?: (dojo: KitDojo) => void;
		onCreateOrJoin?: () => void;
	} = $props();

	const groups = $derived(groupDojos(dojos));
</script>

<div class="flex flex-col p-4 gap-4 md:p-8 md:gap-6">
	<SectionHead eyebrow="You · membership" title="My dōjōs" count={dojos.length}>
		{#snippet right()}
			<Btn size="sm" icon="add-circle" onclick={onCreateOrJoin}>Create or join</Btn>
		{/snippet}
	</SectionHead>

	<Banner kanji="結" tone="neutral" title="A dōjō is how a team shares what it learns.">
		{#if dojos.length}
			You belong to {dojos.length} — your role in each is derived from git and only ever adds
			capability. Working solo needs none of them.
		{:else}
			Your role in each is derived from git and only ever adds capability. Working solo needs none of
			them.
		{/if}
	</Banner>

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
