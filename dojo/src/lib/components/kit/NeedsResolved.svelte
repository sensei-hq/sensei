<script lang="ts">
	import Icon from './Icon.svelte';

	// The resolved marker for a needs-you row (kit K2NeedsResolved) — a success
	// check + a short past-tense label ("approved", "settled", …) that replaces the
	// action set once the viewer has acted.
	let { label }: { label: string } = $props();
</script>

<span class="text-success inline-flex flex-shrink-0 items-center gap-1 whitespace-nowrap text-xs">
	<Icon name="check-circle" size={15} toneClass="text-success" />{label}
</span>
