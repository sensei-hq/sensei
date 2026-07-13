<script lang="ts">
	import type { Snippet } from 'svelte';

	// Section header band inside the main column (mockup DojoHead): a large kanji,
	// an eyebrow, a display title, an optional sub-line, and an optional right slot.
	let {
		kanji,
		eyebrow,
		title,
		sub,
		right
	}: {
		kanji: string;
		eyebrow: string;
		title: string;
		sub?: string;
		right?: Snippet;
	} = $props();
</script>

<div
	class="border-paper-edge flex flex-shrink-0 items-start gap-5 border-b"
	style="padding: 22px 28px 18px"
>
	<span class="kanji text-accent flex-shrink-0" style="font-size: 38px; line-height: 1">{kanji}</span>
	<div class="flex-1" style="min-width: 0">
		<div class="text-ink-mute uppercase" style="font-size: 11px; letter-spacing: 0.18em; margin-bottom: 4px">
			{eyebrow}
		</div>
		<h1 class="display font-normal" style="font-size: 24px; letter-spacing: -0.015em; margin: 0; line-height: 1.05">
			{title}
		</h1>
		{#if sub}
			<p class="text-ink-soft text-sm" style="line-height: 1.55; margin: 6px 0 0; max-width: 680px">{sub}</p>
		{/if}
	</div>
	{#if right}
		<div class="flex-shrink-0">{@render right()}</div>
	{/if}
</div>
