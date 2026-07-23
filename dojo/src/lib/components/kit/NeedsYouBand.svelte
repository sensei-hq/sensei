<script lang="ts">
	import Icon from './Icon.svelte';
	import NeedsRow from './NeedsRow.svelte';
	import EmptyState from './EmptyState.svelte';
	import { type NeedsAction } from './needs';
	import type { KitNeed } from './types';

	// The needs-you band (kit K2NeedsYouBand) — the cross-dōjō "blocked on you"
	// surface: an accent header with the open count, then a row per item. It is a
	// remote control — the viewer acts inline (see NeedsRow), nothing routes them
	// away. When nothing (or nothing unresolved) is waiting, a calm empty state
	// speaks the voice line. `resolved` is a `{ [id]: label }` map; `mobile`
	// switches rows to the stacked phone layout and drops the header hint.
	let {
		items = [],
		resolved,
		title = 'Needs you',
		mobile = false,
		onOpen,
		onAct
	}: {
		items?: KitNeed[];
		resolved?: Record<string, string>;
		title?: string;
		mobile?: boolean;
		onOpen?: (item: KitNeed) => void;
		onAct?: (item: KitNeed, action: NeedsAction) => void;
	} = $props();

	const open = $derived(items.filter((it) => !resolved?.[it.id]).length);
</script>

<div class="border-accent-soft bg-paper-soft overflow-hidden rounded-lg border">
	<div
		class="bg-accent-soft border-accent-soft flex items-center gap-2 border-b"
		style="padding: 12px 16px"
	>
		<Icon name="bell" size={17} toneClass="text-accent" />
		<span class="text-accent text-xs font-semibold uppercase" style="letter-spacing: 0.18em"
			>{title}</span
		>
		<span class="mono text-accent text-xs">{open}</span>
		<span class="flex-1"></span>
		{#if !mobile}
			<span class="mono text-ink-mute hidden text-xs md:inline"
				>{open ? 'act here — nothing routes you away' : 'nothing else is blocked on you'}</span
			>
		{/if}
	</div>
	{#if items.length}
		{#each items as item (item.id)}
			<NeedsRow {item} {resolved} {onOpen} {onAct} stacked={mobile} />
		{/each}
	{:else}
		<EmptyState kanji="静" title="Nothing needs you.">
			Sessions run within the rules you set. sensei will surface only what it can’t decide alone.
		</EmptyState>
	{/if}
</div>
