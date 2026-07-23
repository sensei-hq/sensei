<script lang="ts">
	import StanceDial from './StanceDial.svelte';
	import { stance } from './fixtures';

	// Test-friendly wrapper for StanceDial: records the last (id, value) the dial
	// forwarded through onChange, so a spec can assert selecting a dot fires it.
	let { dial = stance[0] }: { dial?: (typeof stance)[number] } = $props();

	let lastId = $state('');
	let lastValue = $state(-1);
</script>

<StanceDial
	{dial}
	onChange={(id, value) => {
		lastId = id;
		lastValue = value;
	}}
/>
<span data-testid="last-id">{lastId}</span>
<span data-testid="last-value">{lastValue}</span>
