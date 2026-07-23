<script lang="ts">
	import type { Snippet } from 'svelte';
	import KanjiToken from './KanjiToken.svelte';

	// Empty state (kit K2EmptyState) — a faint kanji anchor, one landing line, a
	// calm second sentence, and an optional action. The default copy follows the
	// voice rules (sentence case, no exclamation): "Still listening."
	let {
		kanji = '空',
		title = 'Still listening.',
		action,
		children
	}: { kanji?: string; title?: string; action?: Snippet; children?: Snippet } = $props();
</script>

<div class="flex flex-col items-center text-center" style="padding: 64px 32px; gap: 12px">
	<KanjiToken char={kanji} size="3xl" toneClass="text-ink-faint" />
	<div class="display text-ink text-lg font-normal" style="letter-spacing: -0.01em">{title}</div>
	{#if children}
		<div class="text-ink-soft text-sm" style="line-height: 1.55; max-width: 380px; margin: 0">
			{@render children()}
		</div>
	{/if}
	{#if action}<div style="margin-top: 8px">{@render action()}</div>{/if}
</div>
