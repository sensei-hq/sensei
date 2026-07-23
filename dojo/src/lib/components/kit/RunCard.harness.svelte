<script lang="ts">
	import RunCard from './RunCard.svelte';
	import { runs } from './fixtures';

	// Test-friendly wrapper for RunCard: a click counter so a spec can assert
	// onOpen fires. `flat` / `stacked` exercise the layout variants.
	let {
		run = runs[0],
		flat = false,
		stacked = false
	}: { run?: (typeof runs)[number]; flat?: boolean; stacked?: boolean } = $props();

	let opens = $state(0);
</script>

<RunCard {run} {flat} {stacked} onOpen={() => (opens += 1)} />
<span data-testid="opens">{opens}</span>
