<script lang="ts">
	import Banner from './Banner.svelte';
	import type { BannerTone } from './Banner.svelte';

	// Test-friendly wrapper for Banner: a fixed body child + plain title/tone props.
	let {
		title,
		tone = 'neutral',
		kanji,
		body = 'a calm second sentence'
	}: { title?: string; tone?: BannerTone; kanji?: string; body?: string } = $props();
</script>

<Banner {title} {tone} {kanji}>{body}</Banner>
