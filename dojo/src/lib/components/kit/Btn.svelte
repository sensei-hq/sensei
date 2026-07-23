<script lang="ts" module>
	// variant → surface token classes + the icon/kanji tint. The kit rode on
	// `zs-btn`; dōjō has no zs-* layer, so the button is built from named-token
	// utilities: `primary` is the ink-on-paper CTA (`bg-primary text-on-primary`,
	// matching the shipped console), `ghost` a hairline paper button, `danger` a
	// solid danger fill.
	const VARIANTS = {
		primary: { surface: 'bg-primary text-on-primary border-primary', tint: 'text-on-primary' },
		ghost: { surface: 'bg-paper border-paper-edge text-ink', tint: 'text-accent' },
		danger: { surface: 'bg-danger text-on-primary border-danger', tint: 'text-on-primary' }
	} as const;

	export type BtnVariant = keyof typeof VARIANTS;
	export type BtnSize = 'sm' | 'md';
</script>

<script lang="ts">
	import type { Snippet } from 'svelte';
	import Icon from './Icon.svelte';

	// Canonical button (kit K2Btn). variant: primary | ghost | danger. size sm|md.
	// Optional leading icon (i-glyph) OR kanji (icon wins). Presentational — click
	// is forwarded to the caller's handler.
	let {
		variant = 'primary',
		size = 'md',
		kanji,
		icon,
		title,
		onclick,
		children
	}: {
		variant?: BtnVariant;
		size?: BtnSize;
		kanji?: string;
		icon?: string;
		title?: string;
		onclick?: (e: MouseEvent) => void;
		children?: Snippet;
	} = $props();

	const v = $derived(VARIANTS[variant]);
	const pad = $derived(size === 'sm' ? '6px 12px' : '8px 16px');
	const iconPx = $derived(size === 'sm' ? 15 : 16);
</script>

<button
	type="button"
	{title}
	{onclick}
	class="inline-flex cursor-pointer items-center justify-center gap-2 whitespace-nowrap rounded-lg border font-medium {size ===
	'sm'
		? 'text-xs'
		: 'text-sm'} {v.surface}"
	style="padding: {pad}"
>
	{#if icon}<Icon name={icon} size={iconPx} toneClass={v.tint} />{:else if kanji}<span
			class="kanji {v.tint}">{kanji}</span
		>{/if}{#if children}{@render children()}{/if}
</button>
