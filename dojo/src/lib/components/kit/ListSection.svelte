<script lang="ts">
	import type { Snippet } from 'svelte';
	import Icon from './Icon.svelte';

	// A titled list section (kit K2ListSection): icon + eyebrow title + count +
	// right slot, over a single flush card holding the rows. The one recipe for
	// every "header + rows" block. The mockup's `zs-card-flush` maps to a
	// paper-soft card with a hairline edge and no inner padding (rows own their
	// own padding + dividers).
	let {
		icon,
		iconToneClass = 'text-accent',
		title,
		count,
		countToneClass = 'text-ink-faint',
		right,
		children
	}: {
		icon?: string;
		iconToneClass?: string;
		title: string;
		count?: number | string;
		countToneClass?: string;
		right?: Snippet;
		children?: Snippet;
	} = $props();
</script>

<div>
	<div class="flex items-center gap-2" style="margin-bottom: 12px">
		{#if icon}<Icon name={icon} size={17} toneClass={iconToneClass} />{/if}
		<span class="text-ink text-xs font-semibold uppercase" style="letter-spacing: 0.18em">{title}</span>
		{#if count != null}<span class="mono text-xs {countToneClass}">{count}</span>{/if}
		<span class="flex-1"></span>
		{#if right}{@render right()}{/if}
	</div>
	<div class="bg-paper-soft border-paper-edge overflow-hidden rounded-lg border">
		{#if children}{@render children()}{/if}
	</div>
</div>
