<script lang="ts">
	import type { Snippet } from 'svelte';

	// A one-line status banner used across the console screens (mockup's inline
	// warning/success strips). `tone` picks the token family; `kanji` the glyph;
	// the message is a snippet so callers can inline a mono error string.
	let {
		tone = 'warning',
		kanji = '検',
		children
	}: { tone?: 'warning' | 'success' | 'accent'; kanji?: string; children: Snippet } = $props();

	const shell = $derived(
		tone === 'success'
			? 'bg-success-soft border-success-edge'
			: tone === 'accent'
				? 'bg-accent-soft border-accent-edge'
				: 'bg-warning-soft border-warning-edge'
	);
	const glyph = $derived(
		tone === 'success' ? 'text-success' : tone === 'accent' ? 'text-accent' : 'text-warning'
	);
</script>

<div
	class="text-ink-soft flex items-center gap-2 rounded-xl border text-sm {shell}"
	style="padding: 12px 16px; margin-top: 16px"
>
	<span class="kanji {glyph}">{kanji}</span>
	<span>{@render children()}</span>
</div>
