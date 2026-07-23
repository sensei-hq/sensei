<script lang="ts">
	import Chip from './Chip.svelte';

	// Test-friendly wrapper: exposes Chip's prop surface as plain props and supplies
	// a fixed text child so specs can mount it without authoring a snippet.
	let {
		label = 'chip',
		kanji,
		icon,
		toneClass,
		softClass,
		edgeClass,
		mono = false
	}: {
		label?: string;
		kanji?: string;
		icon?: string;
		toneClass?: string;
		softClass?: string;
		edgeClass?: string;
		mono?: boolean;
	} = $props();
</script>

<Chip {kanji} {icon} {toneClass} {softClass} {edgeClass} {mono}>{label}</Chip>
