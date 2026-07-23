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
	// A single functional kanji — the brand-mark unit. Tone is a token text class
	// (default accent). `w` centers the glyph in a fixed column when given.
	let {
		char,
		size = 'base',
		toneClass = 'text-accent',
		w
	}: { char: string; size?: KanjiSize; toneClass?: string; w?: number } = $props();

	const px = $derived(SIZE_PX[size] ?? SIZE_PX.base);
</script>

<span
	class="kanji inline-block flex-shrink-0 {toneClass}"
	style="font-size: {px}px; line-height: 1{w != null ? `; width: ${w}px; text-align: center` : ''}"
>{char}</span>
