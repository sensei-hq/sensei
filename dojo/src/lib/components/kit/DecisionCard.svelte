<script lang="ts">
	import Icon from './Icon.svelte';
	import Btn from './Btn.svelte';
	import type { KitDecision } from './types';

	// A decision card (kit K2DecisionCard) — sign off a rule. Title · project ·
	// context evidence · age, over an option row. The first option is the primary
	// CTA (leads with a check); the rest are ghost buttons. `onChoose` fires with
	// the chosen option string.
	let {
		decision,
		onChoose
	}: { decision: KitDecision; onChoose?: (option: string) => void } = $props();
</script>

<div class="bg-paper-soft border-paper-edge overflow-hidden rounded-lg border">
	<div class="flex items-start gap-3" style="padding: 16px">
		<Icon name="checklist-minimalistic" size={22} toneClass="text-accent" />
		<div class="flex-1" style="min-width: 0">
			<div class="text-ink text-sm font-medium" style="line-height: 1.3">{decision.title}</div>
			<div class="text-ink-soft text-sm" style="line-height: 1.55; margin-top: 3px">
				{decision.project} · {decision.context}
			</div>
		</div>
		<span class="mono text-ink-faint text-xs">{decision.age}</span>
	</div>
	<div
		class="border-paper-edge bg-paper flex flex-wrap gap-2 border-t"
		style="padding: 12px 16px"
	>
		{#each decision.options as option, i (i)}
			<Btn
				size="sm"
				variant={i === 0 ? 'primary' : 'ghost'}
				icon={i === 0 ? 'check-circle' : undefined}
				onclick={() => onChoose?.(option)}>{option}</Btn
			>
		{/each}
	</div>
</div>
