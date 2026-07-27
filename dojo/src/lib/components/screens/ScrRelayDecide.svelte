<script lang="ts">
	import { SectionHead, DecisionCard, EmptyState } from '$lib/components/kit';
	import type { KitDecision } from '$lib/components/kit/types';

	// Relay · decide (mockup ScrRelayDecide) — the rules waiting on your sign-off,
	// one DecisionCard each. When nothing is pending the screen degrades to the
	// shared EmptyState (sensei stays quiet when there is nothing to decide) rather
	// than a fabricated card list. Presentational: the page supplies the decisions.
	// `onChoose` fires with the decision and the chosen option.
	let {
		decisions = [],
		onChoose
	}: {
		decisions?: KitDecision[];
		onChoose?: (decision: KitDecision, option: string) => void;
	} = $props();
</script>

<div class="flex flex-col p-8 gap-6">
	<SectionHead eyebrow="Relay · decide" title="Rules to sign off" count={decisions.length} />

	{#if decisions.length}
		{#each decisions as decision (decision.id)}
			<DecisionCard {decision} onChoose={(option) => onChoose?.(decision, option)} />
		{/each}
	{:else}
		<EmptyState kanji="静" title="That's everything.">
			Rules a session proposes land here for your sign-off. When there is nothing to decide, sensei
			stays quiet.
		</EmptyState>
	{/if}
</div>
