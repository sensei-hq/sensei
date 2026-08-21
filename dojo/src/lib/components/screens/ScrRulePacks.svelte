<script lang="ts">
	import { SectionHead, Banner, Btn, Icon, Chip, ListSection, RuleRow } from '$lib/components/kit';
	import type { KitRulePack } from '$lib/components/kit/types';
	import { createRulePacks } from '$lib/rulepacks-state.svelte';
	import { untrack } from 'svelte';
	import { SvelteSet } from 'svelte/reactivity';

	// Rule packs (mockup ScrRulePacks) — adoptable rule-pack bundles, split into
	// what you've adopted and what's still available. A pack is a bundle of rules
	// you adopt into your constitution at a scope you choose — a starting point,
	// not a lock (you can drop any single rule later). Each row is expandable: the
	// "N rules" chip is a disclosure that reveals the pack's actual rules, so a
	// viewer can read what a pack contains before adopting it. Presentational: the
	// page supplies the packs (kit fixtures this chunk); adopting/dropping is local
	// state (optimistic) and forwards `onToggle(pack, adopt)` — `adopt` is the DESIRED
	// state (captured before the local flip) — so the page persists it to /v1.
	let {
		packs = [],
		onToggle
	}: { packs?: KitRulePack[]; onToggle?: (pack: KitRulePack, adopt: boolean) => void } = $props();

	// Seed the adopt-toggle state ONCE from the page-load props (a navigation
	// re-mounts the screen), so the adopted/available split persists as the
	// viewer adopts. `untrack` makes that initial capture explicit.
	const packState = untrack(() => createRulePacks(packs));

	// Which packs are showing their rules — local UI state, collapsed by default.
	const expanded = new SvelteSet<string>();

	function toggleExpand(id: string) {
		if (expanded.has(id)) expanded.delete(id);
		else expanded.add(id);
	}

	function adopt(pack: KitRulePack) {
		const desired = !pack.adopted; // captured before the optimistic flip
		packState.toggle(pack.id);
		onToggle?.(pack, desired);
	}
</script>

<div class="flex flex-col p-4 gap-4 md:p-8 md:gap-6">
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
		Adopting a pack adds its rules at the scope you choose. Open a pack to read its rules first —
		you can drop any single rule later, so a pack is a starting point, not a lock.
	</Banner>

	<ListSection
		icon="check-circle"
		iconToneClass="text-success"
		title="Adopted"
		count={packState.adopted.length}
		countToneClass="text-success"
	>
		{#each packState.adopted as pack (pack.id)}
			{@render packRow(pack)}
		{/each}
	</ListSection>

	<ListSection icon="box" title="Available" count={packState.available.length}>
		{#each packState.available as pack (pack.id)}
			{@render packRow(pack)}
		{/each}
	</ListSection>
</div>

{#snippet packRow(pack: KitRulePack)}
	{@const open = expanded.has(pack.id)}
	<div class="border-paper-edge border-b">
		<div class="flex items-center gap-4 px-4 py-3">
			<Icon name="box" size={20} toneClass="text-accent" />
			<div class="flex-1" style="min-width: 0">
				<div class="flex items-center gap-2">
					<span class="text-ink text-sm font-medium">{pack.name}</span>
					<button
						type="button"
						onclick={() => toggleExpand(pack.id)}
						aria-expanded={open}
						aria-label={open ? 'Hide rules' : 'Show rules'}
						class="cursor-pointer bg-transparent"
					>
						<Chip mono>
							{pack.rules.length} rules
							<Icon
								name="alt-arrow-down"
								size={12}
								toneClass="text-ink-mute {open ? 'rotate-180' : ''}"
							/>
						</Chip>
					</button>
				</div>
				<div class="mono text-ink-mute text-xs" style="margin-top: 2px">
					by {pack.by} · {pack.note}
				</div>
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
		{#if open}
			<div class="border-paper-edge bg-paper-soft border-t">
				{#each pack.rules as rule, i (i)}
					<RuleRow rule={{ kanji: pack.kanji, text: rule }} />
				{/each}
			</div>
		{/if}
	</div>
{/snippet}
