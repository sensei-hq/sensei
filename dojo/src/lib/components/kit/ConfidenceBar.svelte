<script lang="ts">
	import Confidence from '../Confidence.svelte';

	// Thin kit wrapper over the shipped `Confidence` bar (kit K2ConfidenceBar).
	// Maps the kit's `v` / `w` props onto the base component's `value` / `width`;
	// the base already tints the fill and value by the confidence band, so no
	// threshold logic is duplicated here.
	let { v, w = 96 }: { v: number; w?: number } = $props();
</script>

<Confidence value={v} width={w} />
