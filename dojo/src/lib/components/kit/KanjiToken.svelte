<script lang="ts" module>
	// Named size steps → px (the kit's brand-glyph geometry — the CJK marks sit
	// off the 8-stop UI type scale on purpose). Font-size for a standalone brand
	// glyph is geometry the type scale doesn't model, so it stays inline.
	const SIZE_PX: Record<string, number> = {
		xs: 11,
		sm: 13,
		base: 15,
		lg: 17,
		xl: 22,
		'2xl': 28,
		'3xl': 40,
		'4xl': 56
	};

	export type KanjiSize = keyof typeof SIZE_PX;
</script>

<script lang="ts">
	import { iconForKanji } from '@sensei/glyphs';
	import { glyphs } from '$lib/glyphs.svelte';

	// A single functional kanji — the brand-mark unit. Tone is a token text class
	// (default accent). `w` centers the glyph in a fixed column when given.
	//
	// It follows the dōjō glyph choice (#133): in icon mode it renders the icon
	// that stands for `char` — its own `icon`, else the shared vocabulary's —
	// at the same size and tone. Every mark follows the mode — one rule across
	// the app and the dōjō (2026-10-27); a kanji that is data (an organisation's,
	// a dōjō's) passes its kind's `icon`. `size="inherit"` adds no font-size, for
	// a mark whose size its `class` states.
	let {
		char,
		size = 'base',
		toneClass = 'text-accent',
		w,
		icon,
		class: extra = ''
	}: {
		char: string;
		size?: KanjiSize | 'inherit';
		toneClass?: string;
		w?: number;
		icon?: string;
		class?: string;
	} = $props();

	const px = $derived(size === 'inherit' ? null : (SIZE_PX[size] ?? SIZE_PX.base));
	const resolved = $derived(icon ?? iconForKanji(char));
	const style = $derived(
		[
			px != null && `font-size: ${px}px`,
			'line-height: 1',
			w != null && `width: ${w}px; text-align: center`
		]
			.filter(Boolean)
			.join('; ')
	);
</script>

{#if glyphs.mode === 'icon' && resolved}
	<span
		data-component="glyph-icon"
		class={['inline-block flex-shrink-0', resolved, toneClass, extra]}
		{style}
		aria-hidden="true"
	></span>
{:else}
	<span class={['kanji inline-block flex-shrink-0', toneClass, extra]} {style}>{char}</span>
{/if}
