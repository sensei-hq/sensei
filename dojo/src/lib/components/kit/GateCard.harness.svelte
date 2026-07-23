<script lang="ts">
	import GateCard from './GateCard.svelte';
	import { gates } from './fixtures';

	// Test-friendly wrapper for GateCard: approve / deny counters so a spec can
	// assert the action handlers fire.
	let { gate = gates[0] }: { gate?: (typeof gates)[number] } = $props();

	let approves = $state(0);
	let denies = $state(0);
</script>

<GateCard {gate} onApprove={() => (approves += 1)} onDeny={() => (denies += 1)} />
<span data-testid="approves">{approves}</span>
<span data-testid="denies">{denies}</span>
