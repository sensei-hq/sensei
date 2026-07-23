<script lang="ts">
	import type { Snippet } from 'svelte';
	import Icon from './Icon.svelte';

	// Generic pill (kit K2Chip). Kanji and icon optional. Tones default to a neutral
	// chip; callers pass token classes (`text-accent`, `bg-accent-soft`,
	// `border-accent-soft`) — never a raw color. `mono` switches the label to the
	// tabular mono face. Content is a snippet so callers compose freely.
	let {
		kanji,
		icon,
		toneClass = 'text-ink-mute',
		softClass = 'bg-paper-mute',
		edgeClass = 'border-transparent',
		mono = false,
		children
	}: {
		kanji?: string;
		icon?: string;
		toneClass?: string;
		softClass?: string;
		edgeClass?: string;
		mono?: boolean;
		children?: Snippet;
	} = $props();
</script>

<span
	class="inline-flex items-center gap-1 whitespace-nowrap rounded-full border text-xs {toneClass} {softClass} {edgeClass} {mono
		? 'mono'
		: ''}"
	style="padding: 3px 9px{mono ? '; letter-spacing: 0.04em' : ''}"
>
	{#if icon}<Icon name={icon} size={13} {toneClass} />{/if}{#if kanji}<span
			class="kanji"
			style="font-size: 12px">{kanji}</span
		>{/if}{#if children}{@render children()}{/if}
</span>
