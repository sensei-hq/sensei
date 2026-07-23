<script lang="ts">
	import { SectionHead, GateCard, EmptyState } from '$lib/components/kit';
	import type { KitGate } from '$lib/components/kit/types';

	// Relay · approve (mockup ScrRelayApprove) — the commands awaiting your
	// approve/deny, one GateCard each. Presentational: the page supplies the gates
	// (kit fixtures this chunk). `onApprove` / `onDeny` fire with the gate.
	// Degrades to an honest empty state when nothing is waiting (DJ1).
	let {
		gates = [],
		onApprove,
		onDeny
	}: {
		gates?: KitGate[];
		onApprove?: (gate: KitGate) => void;
		onDeny?: (gate: KitGate) => void;
	} = $props();
</script>

<div class="flex flex-col" style="padding: 32px; gap: 24px">
	<SectionHead eyebrow="Relay · approve" title="Commands waiting on you" count={gates.length} />

	{#if gates.length}
		{#each gates as gate (gate.id)}
			<GateCard {gate} onApprove={() => onApprove?.(gate)} onDeny={() => onDeny?.(gate)} />
		{/each}
	{:else}
		<EmptyState kanji="静" title="Nothing waiting on you.">
			When a session hits a guarded command it lands here for your approval. Right now every session
			is running within the rules you set.
		</EmptyState>
	{/if}
</div>
