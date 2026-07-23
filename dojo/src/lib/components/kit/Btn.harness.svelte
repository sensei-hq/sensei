<script lang="ts">
	import Btn from './Btn.svelte';
	import type { BtnVariant, BtnSize } from './Btn.svelte';

	// Test-friendly wrapper for Btn: a fixed text label + a plain click count so a
	// spec can assert the label renders and the handler fires.
	let {
		label = 'Approve',
		variant = 'primary',
		size = 'md',
		icon,
		kanji
	}: {
		label?: string;
		variant?: BtnVariant;
		size?: BtnSize;
		icon?: string;
		kanji?: string;
	} = $props();

	let clicks = $state(0);
</script>

<Btn {variant} {size} {icon} {kanji} onclick={() => (clicks += 1)}>{label}</Btn>
<span data-testid="clicks">{clicks}</span>
