<script lang="ts">
	import EnsoRing from '../EnsoRing.svelte';

	// Thin kit wrapper over the shipped `EnsoRing` (kit K2Enso). Kept as a named
	// kit entry so screens compose `kit/Enso` alongside the rest of the kit; it
	// forwards straight through — no duplicated ring geometry.
	let {
		progress,
		label,
		size = 104,
		stroke = 8
	}: { progress: number; label: string; size?: number; stroke?: number } = $props();
</script>

<EnsoRing {progress} {label} {size} {stroke} />
