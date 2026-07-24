<script lang="ts">
	import type { Snippet } from 'svelte';
	import KanjiToken from './KanjiToken.svelte';

	// Shared empty state (kit K2EmptyState) — the ONE calm placeholder every screen
	// reaches for when its content is empty. A faint kanji anchor, one landing line,
	// a calm description sentence, and an optional action, wrapped in a small bordered
	// card centred in the available space. The default copy follows the voice rules
	// (sentence case, no exclamation): "Still listening." Reads well both as a
	// section's sole content (centred in the page) and inline in a flush card.
	let {
		kanji = '空',
		title = 'Still listening.',
		action,
		children
	}: { kanji?: string; title?: string; action?: Snippet; children?: Snippet } = $props();
</script>

<div class="flex flex-col items-center justify-center py-16 px-8">
	<div
		class="border-paper-edge bg-paper-soft flex max-w-sm flex-col items-center rounded-lg border p-8 text-center gap-3"
	>
		<KanjiToken char={kanji} size="3xl" toneClass="text-ink-faint" />
		<div class="display text-ink text-lg font-normal">{title}</div>
		{#if children}
			<div class="text-ink-soft text-sm">{@render children()}</div>
		{/if}
		{#if action}<div class="mt-2">{@render action()}</div>{/if}
	</div>
</div>
