<script lang="ts" module>
	// tone → { fill, edge, kanji-tint } token classes. The mockup used a raw
	// oklch string for the warning edge; here every tone resolves to a named token
	// so the banner stays theme-free.
	const TONES = {
		neutral: { bg: 'bg-paper-soft', edge: 'border-paper-edge', k: 'text-ink-mute' },
		accent: { bg: 'bg-accent-soft', edge: 'border-accent-soft', k: 'text-accent' },
		success: { bg: 'bg-success-soft', edge: 'border-success-soft', k: 'text-success' },
		warning: { bg: 'bg-warning-soft', edge: 'border-warning-soft', k: 'text-warning' }
	} as const;

	export type BannerTone = keyof typeof TONES;
</script>

<script lang="ts">
	import type { Snippet } from 'svelte';
	import KanjiToken from './KanjiToken.svelte';

	// Notice band (kit K2Banner): optional kanji anchor · title · body · right
	// slot. `tone` picks the fill/edge/kanji tint.
	let {
		kanji,
		tone = 'neutral',
		title,
		right,
		children
	}: {
		kanji?: string;
		tone?: BannerTone;
		title?: string;
		right?: Snippet;
		children?: Snippet;
	} = $props();

	const t = $derived(TONES[tone]);
</script>

<div
	class="flex items-start gap-3 rounded-lg border {t.bg} {t.edge}"
	style="padding: 12px 16px"
>
	{#if kanji}<KanjiToken char={kanji} size="lg" toneClass={t.k} />{/if}
	<div class="flex-1" style="min-width: 0">
		{#if title}<div class="text-ink text-sm font-medium" style="line-height: 1.3">{title}</div>{/if}
		{#if children}
			<div class="text-ink-soft text-sm" style="line-height: 1.55; margin-top: {title ? 2 : 0}px">
				{@render children()}
			</div>
		{/if}
	</div>
	{#if right}{@render right()}{/if}
</div>
