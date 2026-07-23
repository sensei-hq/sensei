<script lang="ts">
	import { SectionHead, Banner, Btn, Icon, Chip, ListSection } from '$lib/components/kit';
	import type { KitRulePack } from '$lib/components/kit/types';
	import { createRulePacks } from '$lib/dojo2-rulepacks-state.svelte';
	import { untrack } from 'svelte';

	// Rule packs (mockup ScrRulePacks) — adoptable rule-pack bundles, split into
	// what you've adopted and what's still available. A pack is a bundle of rules
	// you adopt into your constitution at a scope you choose — a starting point,
	// not a lock (you can drop any single rule later). Presentational: the page
	// supplies the packs (kit fixtures this chunk); adopting/dropping is local
	// state this chunk (a later chunk wires it to a /v1 mutation) and also
	// forwards `onToggle(pack)` so a caller can observe the change.
	let {
		packs = [],
		onToggle
	}: { packs?: KitRulePack[]; onToggle?: (pack: KitRulePack) => void } = $props();

	// Seed the adopt-toggle state ONCE from the page-load props (a navigation
	// re-mounts the screen), so the adopted/available split persists as the
	// viewer adopts. `untrack` makes that initial capture explicit.
	const state = untrack(() => createRulePacks(packs));

	function adopt(pack: KitRulePack) {
		state.toggle(pack.id);
		onToggle?.(pack);
	}
</script>

<div class="flex flex-col p-8 gap-6">
	<SectionHead eyebrow="Adopt · not a library" title="Rule packs">
		{#snippet right()}
			<Btn size="sm" variant="ghost" icon="tuning-2">Browse all</Btn>
		{/snippet}
	</SectionHead>

	<Banner
		kanji="束"
		tone="neutral"
		title="Packs are bundles of rules you adopt into your constitution."
	>
		Adopting a pack adds its rules at the scope you choose. You can drop any single rule later — a
		pack is a starting point, not a lock.
	</Banner>

	<ListSection
		icon="check-circle"
		iconToneClass="text-success"
		title="Adopted"
		count={state.adopted.length}
		countToneClass="text-success"
	>
		{#each state.adopted as pack (pack.id)}
			{@render packRow(pack)}
		{/each}
	</ListSection>

	<ListSection icon="box" title="Available" count={state.available.length}>
		{#each state.available as pack (pack.id)}
			{@render packRow(pack)}
		{/each}
	</ListSection>
</div>

{#snippet packRow(pack: KitRulePack)}
	<div class="border-paper-edge flex items-center gap-4 border-b px-4 py-3">
		<Icon name="box" size={20} toneClass="text-accent" />
		<div class="flex-1" style="min-width: 0">
			<div class="flex items-center gap-2">
				<span class="text-ink text-sm font-medium">{pack.name}</span>
				<Chip mono>{pack.count} rules</Chip>
			</div>
			<div class="mono text-ink-mute text-xs" style="margin-top: 2px">by {pack.by} · {pack.note}</div>
		</div>
		{#if pack.adopted}
			<Chip
				icon="check-circle"
				toneClass="text-success"
				softClass="bg-success-soft"
				edgeClass="border-success-soft">adopted</Chip
			>
			<Btn size="sm" variant="ghost" icon="minus-circle" onclick={() => adopt(pack)}>Drop</Btn>
		{:else}
			<Btn size="sm" variant="ghost" icon="add-circle" onclick={() => adopt(pack)}>Adopt</Btn>
		{/if}
	</div>
{/snippet}
