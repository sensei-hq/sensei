<script lang="ts">
	import Btn from './Btn.svelte';
	import { needsActions, type NeedsAction } from './needs';
	import type { KitNeed } from './types';

	// The per-kind action set for a needs-you row (kit K2NeedsActions) — the band
	// is a remote control, so the viewer acts inline. The button set is chosen by
	// the item's kind (gate → approve/deny · conflict → settle · decision → decide
	// · review → open/deny). Clicks stop propagation so acting doesn't also open
	// the row, then forward `onAct(item, action)`.
	let {
		item,
		size = 'sm',
		onAct
	}: {
		item: KitNeed;
		size?: 'sm' | 'md';
		onAct?: (item: KitNeed, action: NeedsAction) => void;
	} = $props();

	const acts = $derived(needsActions(item.kind));
</script>

<div class="flex flex-shrink-0 items-center gap-2">
	{#each acts as a (a.id)}
		<Btn
			{size}
			variant={a.primary ? 'primary' : 'ghost'}
			icon={a.icon}
			onclick={(e) => {
				e.stopPropagation();
				onAct?.(item, a);
			}}>{a.label}</Btn
		>
	{/each}
</div>
