<script lang="ts">
	import BaseSpark from '../Spark.svelte';

	// Thin kit wrapper over the shipped `Spark` (kit K2Spark): the base sparkline
	// inherits `currentColor`, so this just applies a token tint class and forwards
	// the data. No duplicate geometry — the SVG lives once in ../Spark.svelte.
	let { data, toneClass = 'text-accent' }: { data: number[]; toneClass?: string } = $props();
</script>

<span class="inline-flex {toneClass}"><BaseSpark {data} /></span>
