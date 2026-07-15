<script lang="ts">
	import type { Snippet } from 'svelte';

	// Small pill label. `toneClass` tints the text (a token utility like
	// text-accent); the soft fill comes from the matching *-soft token via
	// currentColor mix, so callers only pass a text token.
	let {
		toneClass = 'text-ink-mute',
		children
	}: { toneClass?: string; children: Snippet } = $props();
</script>

<span
	class="mono border-paper-edge inline-flex items-center rounded-full border {toneClass}"
	style="padding: 2px 8px; font-size: 10.5px; letter-spacing: 0.02em"
>
	{@render children()}
</span>
