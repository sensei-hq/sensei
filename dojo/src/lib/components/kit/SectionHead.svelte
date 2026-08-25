<script lang="ts">
	import type { Snippet } from 'svelte';
	import KanjiToken from './KanjiToken.svelte';

	// The one section header (kit K2SectionHead): optional kanji · eyebrow · title
	// · optional description · optional count · optional right slot. Eyebrow uses
	// the tracked-uppercase micro-label treatment; title is the display face at
	// the section size.
	//
	// `description` is the screen's standing explanation. It used to live in a
	// neutral `Banner` under this header, which meant every screen permanently
	// showed a notice band — so a real warning Banner had to compete with ambient
	// ones. The mockups (triage-stage1.png, scopes-stage7.png) put the glyph in
	// this header and the prose directly beneath the title, with no card.
	let {
		kanji,
		eyebrow,
		title,
		description,
		count,
		right
	}: {
		kanji?: string;
		eyebrow?: string;
		title: string;
		description?: string;
		count?: number | string;
		right?: Snippet;
	} = $props();
</script>

<div class="border-paper-edge flex items-baseline gap-3 border-b" style="padding-bottom: 12px">
	{#if kanji}<KanjiToken char={kanji} size="2xl" />{/if}
	<div style="min-width: 0">
		{#if eyebrow}
			<div
				class="text-ink-mute text-xs font-semibold uppercase"
				style="letter-spacing: 0.18em; margin-bottom: 4px"
			>
				{eyebrow}
			</div>
		{/if}
		<h2
			class="display text-xl font-normal"
			style="letter-spacing: -0.015em; margin: 0; line-height: 1.1"
		>
			{title}
		</h2>
		{#if description}
			<p class="text-ink-soft mt-2 mb-0 max-w-[76ch] text-sm" style="line-height: 1.55">
				{description}
			</p>
		{/if}
	</div>
	{#if count != null}<span class="mono text-ink-faint text-xs">{count}</span>{/if}
	<span class="flex-1"></span>
	{#if right}{@render right()}{/if}
</div>
