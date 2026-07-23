<script lang="ts">
	import RuleRow from './RuleRow.svelte';
	import type { KitRule } from './types';

	// Test-friendly wrapper for RuleRow: fixed rule + click counters so a spec can
	// assert the toggle / jump / edit handlers fire. `withToggle` swaps the leading
	// kanji for the include checkbox.
	let {
		rule = { kanji: '守', text: 'Never log tokens or PII, even at debug level', hard: true, level: 'Company' },
		included = true,
		showLevel = false,
		withToggle = false,
		withJump = false,
		withEdit = false
	}: {
		rule?: KitRule;
		included?: boolean;
		showLevel?: boolean;
		withToggle?: boolean;
		withJump?: boolean;
		withEdit?: boolean;
	} = $props();

	let toggles = $state(0);
	let jumps = $state(0);
	let edits = $state(0);
</script>

<RuleRow
	{rule}
	{included}
	{showLevel}
	onToggle={withToggle ? () => (toggles += 1) : undefined}
	onJump={withJump ? () => (jumps += 1) : undefined}
	onEdit={withEdit ? () => (edits += 1) : undefined}
/>
<span data-testid="toggles">{toggles}</span>
<span data-testid="jumps">{jumps}</span>
<span data-testid="edits">{edits}</span>
