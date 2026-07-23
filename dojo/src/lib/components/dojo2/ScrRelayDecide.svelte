<script lang="ts">
	import { SectionHead, DecisionCard, Banner } from '$lib/components/kit';
	import type { KitDecision } from '$lib/components/kit/types';

	// Relay · decide (mockup ScrRelayDecide) — the rules waiting on your sign-off,
	// one DecisionCard each, closed by the quiet "that's everything" banner (sensei
	// stays quiet when there is nothing to decide). Presentational: the page
	// supplies the decisions (kit fixtures this chunk). `onChoose` fires with the
	// decision and the chosen option.
	let {
		decisions = [],
		onChoose
	}: {
		decisions?: KitDecision[];
		onChoose?: (decision: KitDecision, option: string) => void;
	} = $props();
</script>

<div class="flex flex-col" style="padding: 32px; gap: 24px">
	<SectionHead eyebrow="Relay · decide" title="Rules to sign off" count={decisions.length} />

	{#each decisions as decision (decision.id)}
		<DecisionCard {decision} onChoose={(option) => onChoose?.(decision, option)} />
	{/each}

	<Banner kanji="静" tone="neutral" title="That's everything.">
		When there is nothing to decide, sensei stays quiet.
	</Banner>
</div>
