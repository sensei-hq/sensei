<script lang="ts">
	import DecisionCard from './DecisionCard.svelte';
	import { decisions } from './fixtures';

	// Test-friendly wrapper for DecisionCard: records the last chosen option so a
	// spec can assert onChoose forwards it.
	let { decision = decisions[0] }: { decision?: (typeof decisions)[number] } = $props();

	let chosen = $state('');
</script>

<DecisionCard {decision} onChoose={(o) => (chosen = o)} />
<span data-testid="chosen">{chosen}</span>
