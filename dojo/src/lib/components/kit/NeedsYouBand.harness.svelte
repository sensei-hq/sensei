<script lang="ts">
	import NeedsYouBand from './NeedsYouBand.svelte';
	import { needsYou } from './fixtures';
	import type { KitNeed } from './types';

	// Test-friendly wrapper for NeedsYouBand: records the last (need.id, action.id)
	// forwarded through onAct so a spec can assert the per-kind action set (gate →
	// approve/deny, conflict → settle, decision → decide, review → open) fires the
	// right callback. `resolved` swaps a row's actions for the resolved marker.
	let {
		items = needsYou,
		resolved
	}: { items?: KitNeed[]; resolved?: Record<string, string> } = $props();

	let lastNeed = $state('');
	let lastAction = $state('');
	let opened = $state('');
</script>

<NeedsYouBand
	{items}
	{resolved}
	onOpen={(it) => (opened = it.id)}
	onAct={(it, a) => {
		lastNeed = it.id;
		lastAction = a.id;
	}}
/>
<span data-testid="last-need">{lastNeed}</span>
<span data-testid="last-action">{lastAction}</span>
<span data-testid="opened">{opened}</span>
