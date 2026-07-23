<script lang="ts">
	import SectionHead from './SectionHead.svelte';

	// Test-friendly wrapper for SectionHead: plain props only (the right slot is
	// omitted so the spec can assert the eyebrow/title/count without a snippet).
	let {
		title,
		eyebrow,
		kanji,
		count
	}: { title: string; eyebrow?: string; kanji?: string; count?: number | string } = $props();
</script>

<SectionHead {title} {eyebrow} {kanji} {count} />
